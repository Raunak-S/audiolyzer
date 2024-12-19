mod inputs;

use audiolyzer::display::{DiscreteStrategy, DisplayStrategyFactory, LineStrategy, PointStrategy};
use audiolyzer::fft::*;

use std::{
    io,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use inputs::{events::Events, key::Key, InputEvent};

use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    style::{Color, Style},
    widgets::{
        canvas::{Canvas, Line, Points},
        BarChart, Block, Borders, Clear,
    },
    Terminal,
};

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Optional name to operate on
    #[arg(long, default_value_t = 44100)]
    sample_rate: u32,

    /// Sets a custom config file
    #[arg(long, default_value_t = 22050)]
    bins: usize,

    /// Turn debugging information on
    #[arg(long, default_value_t = 0.7)]
    smoothing_constant: f64,

    #[arg(long, default_value_t = 60)]
    fps: u8,

    #[arg(long, default_value_t = 20)]
    min_freq: u16,

    #[arg(long, default_value_t = 20000)]
    max_freq: u16,

    #[arg(long, default_value_t = String::from("DISCRETE"))]
    display_mode: String,

    #[arg(long, default_value_t = 1024)]
    fft_size: u32,
}

#[derive(Clone, Debug)]
struct StreamOutput {
    data: Vec<f32>,
}

fn calc_avg_tick(
    tickindex: &mut usize,
    ticksum: &mut f32,
    ticklist: &mut [f32],
    newtick: f32,
) -> f32 {
    *ticksum -= ticklist[*tickindex];
    *ticksum += newtick;
    ticklist[*tickindex] = newtick;
    *tickindex = (*tickindex + 1) % 100;

    *ticksum / (ticklist.len() as f32)
}

fn normalize_db(value: f64) -> f64 {
    let max_val = -25f64;
    let min_val = -85f64;

    let normal_val = (value - min_val) / (max_val - min_val);

    if normal_val < 0. {
        0.
    } else if normal_val > 1. {
        1.
    } else {
        normal_val
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let SAMPLE_RATE: u32 = args.sample_rate;
    let BINS: usize = args.bins; // TODO: rename to "bands" and change to work for octave bands
    let S: f64 = args.smoothing_constant;
    let FPS: u8 = args.fps;
    let MIN_FREQ: u16 = args.min_freq;
    let MAX_FREQ: u16 = args.max_freq;
    let DISPLAY_MODE: &str = args.display_mode.as_str();
    let fft_size = args.fft_size;
    let freq_step = f64::from(SAMPLE_RATE) / f64::from(fft_size);

    let data_lock = Arc::new(Mutex::new(StreamOutput { data: vec![] }));
    let main_data_lock = data_lock.clone();
    let host = cpal::default_host();
    let device = host
        .input_devices()
        .unwrap()
        .find(|possible_device| possible_device.name().unwrap() == "BlackHole 2ch")
        .unwrap();
    let custom_config = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(SAMPLE_RATE), // default sample rate 44100
        buffer_size: cpal::BufferSize::Fixed(fft_size), // default buffer size cpal::BufferSize::Default
    };

    let stream = device
        .build_input_stream(
            &custom_config.into(),
            //&default_config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // react to stream events and read or write stream data here.
                match data_lock.lock() {
                    Ok(mut streamoutput) => streamoutput.data = data.to_vec(),
                    _ => (),
                }
            },
            move |err| {
                // react to errors here.
                eprintln!("{err}");
                panic!()
            },
            None
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
    let mut fft_engine = FFTEngine::new(SAMPLE_RATE, BINS, S, WindowType::Blackman);

    let mut start = Instant::now();
    let mut tickindex = 0usize;
    let mut ticksum = 0f32;
    let mut ticklist = [0f32; 100];

    loop {
        let result = match events.next()? {
            InputEvent::Input(key) => key,
            InputEvent::Tick => {
                let data = match main_data_lock.lock() {
                    Ok(mut res) => res.data.clone(),
                    _ => continue,
                };

                fft_engine.set_src_buf(&data);

                fft_engine.apply_window();
                fft_engine.apply_fft();

                let newtick = start.elapsed().as_secs_f32();
                let fps =
                    1f32 / calc_avg_tick(&mut tickindex, &mut ticksum, &mut ticklist, newtick);
                start = Instant::now();

                let canvas = Canvas::default()
                    .block(
                        Block::default()
                            .title(format!(
                                "audiolyzer - Window: {:?} - FPS: {:?}",
                                fft_engine.get_window(),
                                fps
                            ))
                            .borders(Borders::ALL),
                    )
                    .x_bounds([MIN_FREQ.into(), MAX_FREQ.into()])
                    .y_bounds([0.0, 10.0])
                    .paint(|ctx| {
                        let mut freq_data = fft_engine.get_bins();
                        freq_data
                            .iter_mut()
                            .for_each(|x| *x = normalize_db(*x) * 10.);
                        let s = DisplayStrategyFactory::get_display_strategy(DISPLAY_MODE);
                        s.render(ctx, &freq_data, freq_step);
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
            let new_idx = ((fft_engine.get_window() as usize) + 1) % 4;
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
