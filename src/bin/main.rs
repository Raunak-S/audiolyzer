mod inputs;

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

const BINS: usize = 30;
const BIN: Vec<f64> = Vec::new();
const SAMPLE_RATE: u32 = 44100;
const S: f64 = 0.001;
const FPS: u8 = 60;

//data: &[f32], _: &cpal::InputCallbackInfo
#[derive(Clone, Debug)]
struct StreamOutput {
    data: Vec<f32>,
}

// display data helper functions

// creates display data vector for the canvas widget
fn create_canvas_data(
    bins: [Vec<f64>; BINS],
    prev_data_set: &mut Vec<f64>,
    smoothing: f64,
) -> Vec<Line> {
    // B_i' = B_(i-1)' * s' + B_i * (1 - s')
    // s' = s ** (1 / R)
    // R = NUM_OF_SAMPLES / SAMPLE_RATE

    let mut display_vec: Vec<Line> = vec![];
    for bin in bins.iter().enumerate() {
        let y_value_raw = bin.1.iter().copied().fold(1., f64::max).log10();
        let y_value_final = if y_value_raw > prev_data_set[bin.0] {
            y_value_raw
        } else {
            prev_data_set[bin.0] * smoothing + y_value_raw * (1. - smoothing)
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

    let mut prev_data_set = vec![0f64; BINS];

    let tick_rate = Duration::from_millis(1000 / u64::try_from(FPS)?);
    let events = Events::new(tick_rate);
    let mut file = std::fs::File::create("txt/output.txt").unwrap();
    loop {
        let result = match events.next()? {
            InputEvent::Input(key) => key,
            InputEvent::Tick => {
                let data = match main_data_lock.lock() {
                    Ok(mut res) => {
                        let ret = res.clone();
                        res.data.clear();
                        ret
                    }
                    _ => continue,
                };
                let mut real_planner = RealFftPlanner::<f64>::new();

                // Vector holding all of the samples
                let data = data.data;

                let mut windowed_data = vec![0f32; data.len()];
                if !(1 < data.len()) {
                    continue;
                }
                let window = apodize::hanning_iter(data.len())
                    .map(|f| f as f32)
                    .collect::<Vec<f32>>();

                for (windowed, (window, data)) in
                    windowed_data.iter_mut().zip(window.iter().zip(data.iter()))
                {
                    *windowed = *window * *data;
                }

                let data = windowed_data;

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

                /*
                 *
                 * Map the calculated frequencies into specific bins
                 *
                 * */

                for val in spectrum.iter().enumerate() {
                    //if val.0>spectrum.len()/2-1 {break;}
                    if freq_step * val.0 as f64 >= (SAMPLE_RATE / 2) as f64 {
                        continue;
                    }
                    bins[((freq_step * val.0 as f64 / (SAMPLE_RATE / 2) as f64).powf(1. / 2.)
                        * BINS as f64) as usize]
                        .push(val.1.norm_sqr());
                }

                file.write_all(
                    format!(
                        "{:?}\n{:?}\n",
                        data.len(),
                        bins.to_owned().map(|ele| { ele.len() })
                    )
                    .as_bytes(),
                )
                .unwrap();
                let placeholder_vec: Vec<Line> =
                    create_canvas_data(bins, &mut prev_data_set, S.powf(1. / freq_step));

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
