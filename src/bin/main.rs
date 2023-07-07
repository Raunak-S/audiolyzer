mod inputs;

use audiolyzer::fft::*;

use std::{
    io::{self, Write},
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
use realfft::RealFftPlanner;
use tui::{
    backend::CrosstermBackend,
    style::Color,
    widgets::{
        canvas::{Canvas, Line},
        Block, Borders,
    },
    Terminal,
};

const BINS: usize = 50;
const SAMPLE_RATE: u32 = 44100;
const S: f64 = 0.01;
const FPS: u8 = 60;

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
fn create_bar_data(bins: [Vec<f64>; BINS], prev_data_set: &mut Vec<f64>) -> Vec<Line> {
    todo!()
    // // freq_ranges holds strings of tui labels so that they are not
    // // dropped before being used in the bar chart display
    // let freq_ranges: Vec<String> =
    //     (0..bins.len()).map(|label| label.to_string()).collect();
    // for bin in bins.iter().enumerate() {
    //     display_vec.push((
    //         freq_ranges[bin.0].as_str(),
    //         0u64.max(
    //             (prev_data_set[bin.0] * S_PRIME
    //                 + bin
    //                     .1
    //                     .iter()
    //                     .copied()
    //                     .fold(f64::NEG_INFINITY, f64::max)
    //                     .log2()
    //                     * (1. - S_PRIME)) as u64,
    //         ),
    //     ));
    // }

    // let bar = BarChart::default()
    //     .block(Block::default().title("audiolyzer").borders(Borders::ALL))
    //     .bar_width(3)
    //     .bar_gap(1)
    //     .bar_style(Style::default().fg(Color::Yellow))
    //     .value_style(Style::default().bg(Color::Yellow))
    //     .label_style(Style::default())
    //     .data(&display_vec[..])
    //     .max(20);

    // display_vec
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data_lock = Arc::new(Mutex::new(StreamOutput { data: vec![0f32] }));
    let main_data_lock = data_lock.clone();
    let host = cpal::default_host();
    let device = host.default_input_device().unwrap();

    let custom_config = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(SAMPLE_RATE), // default sample rate 44100
        buffer_size: cpal::BufferSize::Default,
    };

    let stream = device
        .build_input_stream(
            &custom_config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // react to stream events and read or write stream data here.
                data_lock.lock().unwrap().data.extend(data.iter());
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
    let mut file = std::fs::File::create("txt/output.txt").unwrap();
    let mut fft_engine = FFTEngine::new(SAMPLE_RATE, BINS, S);
    loop {
        let result = match events.next()? {
            InputEvent::Input(key) => key,
            InputEvent::Tick => {
                let data = match main_data_lock.lock() {
                    Ok(mut res) => {
                        let ret = res.clone();
                        res.data.clear();
                        ret.data
                    }
                    _ => continue,
                };

                fft_engine.push_samples(&data);
                fft_engine.apply_hanning_window();
                fft_engine.apply_fft();

                // file.write_all(
                //     format!(
                //         "{:?}\n{:?}\n",
                //         data.len(),
                //         &fft_engine.get_curr_data().to_owned()
                //     )
                //     .as_bytes(),
                // )
                // .unwrap();

                let placeholder_vec: Vec<Line> = create_canvas_data(&fft_engine.get_bins());

                let canvas = Canvas::default()
                    .block(Block::default().title("audiolyzer").borders(Borders::ALL))
                    .x_bounds([0.0, BINS as f64])
                    .y_bounds([0.0, 5.0])
                    .paint(|ctx| {
                        for line in &placeholder_vec {
                            ctx.draw(line);
                        }
                    });

                terminal
                    .draw(|f| {
                        let size = f.size();
                        f.render_widget(canvas, size);
                    })
                    .unwrap();

                Key::Unknown
            }
        };

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
