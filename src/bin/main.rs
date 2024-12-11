mod inputs;

use audiolyzer::fft::*;

use std::{
    io,
    sync::{Arc, Mutex},
    time::Duration,
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use inputs::{events::Events, key::Key, InputEvent};
use tui::{
    backend::CrosstermBackend,
    style::Color,
    style::Style,
    widgets::{
        canvas::{Canvas, Line},
        BarChart, Block, Borders,
    },
    Terminal,
};

const SAMPLE_RATE: u32 = 44100;
const BINS: usize = 22050; // TODO: rename to "bands" and change to work for octave bands
const S: f64 = 0.00001;
const FPS: u8 = 60;
const MIN_FREQ: u16 = 20;
const MAX_FREQ: u16 = 20000;

#[derive(Clone, Debug)]
struct StreamOutput {
    data: Vec<f32>,
}

// display data helper functions

// creates display data vector for the canvas widget
fn create_canvas_data(bins: &Vec<f64>) -> Vec<Line> {
    let mut display_vec: Vec<Line> = vec![];
    for bin in bins.iter().enumerate() {
        display_vec.push(Line {
            x1: bin.0 as f64,
            y1: 0.0,
            x2: bin.0 as f64,
            y2: bin.1.to_owned(),
            color: Color::White,
        });
    }
    display_vec
}

// creates display data vector for the bar widget
fn create_bar_data(bins: Vec<f64>) -> Vec<(String, u64)> {
    // freq_ranges holds strings of tui labels so that they are not
    // dropped before being used in the bar chart display
    let mut display_vec = vec![];
    let freq_ranges: Vec<String> = (0..bins.len()).map(|label| label.to_string()).collect();
    for bin in bins.iter().enumerate() {
        display_vec.push((freq_ranges[bin.0].clone(), *bin.1 as u64));
    }
    display_vec
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data_lock = Arc::new(Mutex::new(StreamOutput { data: vec![] }));
    let main_data_lock = data_lock.clone();
    let host = cpal::default_host();
    let device = host
        .devices()
        .unwrap()
        .find(|possible_device| possible_device.name().unwrap() == "BlackHole 2ch")
        .unwrap();
    let custom_config = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(SAMPLE_RATE), // default sample rate 44100
        buffer_size: cpal::BufferSize::Fixed(1024), // default buffer size cpal::BufferSize::Default
    };

    let stream = device
        .build_input_stream(
            &custom_config.into(),
            //&default_config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // react to stream events and read or write stream data here.
                match data_lock.lock() {
                    Ok(mut streamoutput) => streamoutput.data = data.to_vec(),
                    _ => ()
                }
            },
            move |err| {
                // react to errors here.
                eprintln!("{err}");
                panic!()
            },
        )
        .unwrap();
    stream.play().unwrap();

    enable_raw_mode().unwrap();
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    let tick_rate = Duration::from_millis(1000 / u64::try_from(FPS)?);
    let events = Events::new(tick_rate);
    let mut fft_engine = FFTEngine::new(SAMPLE_RATE, BINS, S);
    loop {
        let result = match events.next()? {
            InputEvent::Input(key) => key,
            InputEvent::Tick => {
                let data = match main_data_lock.lock() {
                    Ok(mut res) => {
                        res.data.clone()
                    }
                    _ => continue,
                };

                fft_engine.push_samples(&data);

                // let mut sample_debug = vec![];
                // for bin in fft_engine.get_curr_data().iter().enumerate() {
                //     sample_debug.push(Line {
                //         x1: bin.0 as f64,
                //         y1: 0.0,
                //         x2: bin.0 as f64,
                //         y2: bin.1.to_owned() as f64,
                //         color: Color::White,
                //     });
                // }
                // let sample_debug = Canvas::default()
                //     .block(Block::default().title("audiolyzer").borders(Borders::ALL))
                //     .x_bounds([0.0, 1350.0])
                //     .y_bounds([-1.0, 1.0])
                //     .paint(|ctx| {
                //         for line in &sample_debug {
                //             ctx.draw(line);
                //         }
                //     });

                fft_engine.apply_window();
                fft_engine.apply_fft();

                let placeholder_vec: Vec<Line> = create_canvas_data(&fft_engine.get_bins());

                let canvas = Canvas::default()
                    .block(Block::default()
                    .title(format!("audiolyzer - {:?}", fft_engine.get_window()))
                    .borders(Borders::ALL))
                    .x_bounds([MIN_FREQ.into(), MAX_FREQ.into()])
                    .y_bounds([0.0, 70.0])
                    .paint(|ctx| {
                        for line in &placeholder_vec {
                            ctx.draw(line);
                        }
                    });

                // let display_vec = create_bar_data(fft_engine.get_bins());
                // let barchart_data = display_vec
                //     .iter()
                //     .map(|ele| (ele.0.as_str(), ele.1))
                //     .collect::<Vec<(&str, u64)>>();
                // let bar = BarChart::default()
                //     .block(Block::default().title("audiolyzer").borders(Borders::ALL))
                //     .bar_width(3)
                //     .bar_gap(1)
                //     .bar_style(Style::default().fg(Color::Yellow))
                //     .value_style(Style::default().bg(Color::Yellow))
                //     .label_style(Style::default())
                //     .data(&barchart_data)
                //     .max(20);

                terminal
                    .draw(|f| {
                        let size = f.size();
                        f.render_widget(canvas, size);
                    })
                    .unwrap();

                Key::Unknown
            }
        };

        if result.is_right_arrow() {
            let windows = [
                WindowType::Hanning,
                WindowType::Hamming,
                WindowType::Blackman,
                WindowType::Nuttall,
            ];
            let new_idx = ((fft_engine.get_window() as usize)+1) % 4;
            fft_engine.set_window(windows[new_idx].clone());
        }

        if result.is_exit() {
            break;
        }
    }

    // restore terminal
    disable_raw_mode().unwrap();
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .unwrap();
    terminal.show_cursor().unwrap();

    Ok(())
}
