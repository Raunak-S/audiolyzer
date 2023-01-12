mod inputs;

use std::{
    io::{self, Write},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, SampleRate, InputCallbackInfo, InputStreamTimestamp, StreamInstant,
};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use inputs::{events::Events, key::Key, InputEvent};
use realfft::{num_traits::Pow, RealFftPlanner};
use tui::{
    backend::CrosstermBackend,
    style::{Color, Modifier, Style},
    symbols,
    text::Span,
    widgets::{
        canvas::{Canvas, Line},
        Axis, BarChart, Block, Borders, Chart, Dataset, GraphType,
    },
    Terminal,
};

use std::sync::mpsc::channel;

const BINS: usize = 70;
const BIN: Vec<f64> = Vec::new();
const SAMPLE_RATE: u32 = 44100;
const S_PRIME: f64 = 0.5;
const FPS: u8 = 30;

//data: &[f32], _: &cpal::InputCallbackInfo
#[derive(Clone, Debug)]
struct StreamOutput {
    callback: Option<StreamInstant>,
    capture: Option<StreamInstant>,
    data: Vec<f32>,
}


// display data helper functions

// creates display data vector for the canvas widget
fn create_canvas_data(bins: [Vec<f64>; BINS], freq_step: f64, prev_data_set: &mut Vec<f64>) -> Vec<Line> {

    // B_i' = B_(i-1)' * s' + B_i * (1 - s')

    let mut display_vec: Vec<Line> = vec![];
    for bin in bins.iter().enumerate() {
        let y_value_raw = bin
        .1
        .iter()
        .copied()
        .fold(1., f64::max)
        .log10();
        let y_value_final = if y_value_raw > prev_data_set[bin.0] {
            y_value_raw
        } else {
            prev_data_set[bin.0] * S_PRIME
                + y_value_raw * (1. - S_PRIME)
        };
        prev_data_set[bin.0] = y_value_final;
        display_vec.push(Line {
            x1: bin.0 as f64,
            y1: 0.0,
            x2: bin.0 as f64,
            y2: y_value_final,
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
    let data_lock = Arc::new(Mutex::new(StreamOutput {
        data: vec![0f32],
        callback: None,
        capture: None,
    }));
    let main_data_lock = data_lock.clone();
    std::thread::spawn(move || {
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
                move |data: &[f32], callback_info: &cpal::InputCallbackInfo| {
                    // react to stream events and read or write stream data here.
                    *data_lock.lock().unwrap() = StreamOutput {
                        data: data.to_owned(),
                        callback: Some(callback_info.timestamp().callback),
                        capture: Some(callback_info.timestamp().capture),
                    };
                },
                move |err| {
                    // react to errors here.
                    eprintln!("{err}");
                    panic!()
                },
            )
            .unwrap();
        stream.play().unwrap();
        std::thread::park();
    });

    enable_raw_mode().unwrap();
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut prev_data_set = vec![0f64; BINS];

    let tick_rate = Duration::from_millis(1000/u64::try_from(FPS)?);
    let events = Events::new(tick_rate);
    let mut file = std::fs::File::create("txt/output.txt").unwrap();
    loop {
        let result = match events.next()? {
            InputEvent::Input(key) => key,
            InputEvent::Tick => {
                let data = match main_data_lock.lock() {
                    Ok(res) => res.clone(),
                    _ => continue,
                };
                let mut real_planner = RealFftPlanner::<f64>::new();
                
                let callback_info = data.clone();
                let data = data.data;

                // create a FFT
                let r2c = real_planner.plan_fft_forward(data.len());
                // make input and output vectors
                let mut spectrum = r2c.make_output_vec();
                let mut arr: Vec<f64> = data.iter().map(|val| *val as f64).collect();
                
                r2c.process(&mut arr[..], &mut spectrum).unwrap();
                
                let mut bins = [BIN; BINS];

                // let freq_step = if !callback_info.callback.is_none() {
                //     callback_info.callback.unwrap().duration_since(&callback_info.capture.unwrap());
                // }
                
                let freq_step = f64::try_from(SAMPLE_RATE)? / data.len() as f64;
                
                // B_i = ((f_i / f_max) ** (1 / gamma)) * B_max
                
                
                
                for val in spectrum.iter().enumerate() {
                    //if val.0>spectrum.len()/2-1 {break;}
                    if freq_step * val.0 as f64 >= 16000. {continue;}
                    bins[((freq_step * val.0 as f64 / (16000 as f64))
                    .powf(1. / 2.)
                    * BINS as f64) as usize]
                    .push(val.1.norm_sqr());
                }
                
                file.write_all(format!("{:?}\n{:?}\n", data.len(), bins.to_owned().map(|ele| {ele.len()})).as_bytes()).unwrap();
                let placeholder_vec: Vec<Line> = create_canvas_data(bins, freq_step, &mut prev_data_set);
                
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
