mod inputs;

use std::{
    io,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, SampleRate,
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
    widgets::{Axis, BarChart, Block, Borders, Chart, Dataset, GraphType},
    Terminal,
};

use std::sync::mpsc::channel;

use audiolyzer::audio_thread;

const BINS: usize = 30;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data_lock = Arc::new(Mutex::new(vec![0f32]));
    let main_data_lock = data_lock.clone();
    std::thread::spawn(move || {
        let host = cpal::default_host();
        let device = host.default_input_device().unwrap();

        let custom_config = cpal::StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(22050), // default sample rate 44100
            buffer_size: cpal::BufferSize::Default,
        };

        let stream = device
            .build_input_stream(
                &custom_config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // react to stream events and read or write stream data here.
                    *data_lock.lock().unwrap() = data.to_owned();
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

    // B_i' = B_(i-1)' * s' + B_i * (1 - s')

    let mut prev_data_set = vec![0f64; BINS];
    let s_prime = 0.5;

    let tick_rate = Duration::from_millis(34);
    let events = Events::new(tick_rate);

    loop {
        let result = match events.next()? {
            InputEvent::Input(key) => key,
            InputEvent::Tick => {
                let data = match main_data_lock.lock() {
                    Ok(res) => res.clone(),
                    _ => continue,
                };
                let freq_step = 44100f64 / data.len() as f64;
                let mut real_planner = RealFftPlanner::<f64>::new();

                // create a FFT
                let r2c = real_planner.plan_fft_forward(data.len());
                // make input and output vectors
                let mut spectrum = r2c.make_output_vec();

                let mut arr: Vec<f64> = data.iter().map(|val| *val as f64).collect();

                r2c.process(&mut arr[..], &mut spectrum).unwrap();

                let mut display_vec = vec![];

                let mut bins: [Vec<f64>; BINS] = Default::default();

                // B_i = ((f_i / f_max) ** (1 / gamma)) * B_max

                for val in spectrum.iter().enumerate() {
                    //if val.0>spectrum.len()/2-1 {break;}
                    bins[((freq_step * val.0 as f64 / (freq_step * spectrum.len() as f64))
                        .powf(1. / 2.)
                        * BINS as f64) as usize]
                        .push(val.1.norm_sqr());
                }

                for bin in bins.iter().enumerate() {
                    display_vec.push((
                        "1",
                        0u64.max(
                            (prev_data_set[bin.0] * s_prime
                                + bin
                                    .1
                                    .iter()
                                    .copied()
                                    .fold(f64::NEG_INFINITY, f64::max)
                                    .log2()
                                    * (1. - s_prime)) as u64,
                        ),
                    ));
                }

                prev_data_set = display_vec.iter().map(|val| val.1 as f64).collect();

                let bar = BarChart::default()
                    .block(
                        Block::default()
                            .title(data.len().to_string())
                            .borders(Borders::ALL),
                    )
                    .bar_width(3)
                    .bar_gap(1)
                    .bar_style(Style::default().fg(Color::Yellow).bg(Color::Red))
                    .value_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
                    .label_style(Style::default())
                    .data(&display_vec[..])
                    .max(20);

                terminal
                    .draw(|f| {
                        let size = f.size();
                        f.render_widget(bar, size);
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
