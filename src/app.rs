use crate::{
    display::{DiscreteStrategy, DisplayStrategyFactory, LineStrategy, PointStrategy},
    fft::*,
    inputs::{events::Events, key::Key, InputEvent},
};
use clap::{Parser, Subcommand};

use std::{
    io,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Stream,
};

use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{
        canvas::{Canvas, Line, Points},
        BarChart, Block, Borders, Clear, Paragraph,
    },
    Terminal,
};

#[derive(Clone, Debug)]
struct StreamOutput {
    data: Vec<f32>,
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Optional name to operate on
    #[arg(long, default_value_t = 44100)]
    pub sample_rate: u32,

    /// Sets a custom config file
    #[arg(long, default_value_t = 22050)]
    pub bins: usize, // TODO: rename to "bands" and change to work for octave bands

    /// Turn debugging information on
    #[arg(long, default_value_t = 0.7)]
    pub smoothing_constant: f64,

    #[arg(long, default_value_t = 60)]
    pub fps: u8,

    #[arg(long, default_value_t = 20)]
    pub min_freq: u16,

    #[arg(long, default_value_t = 20000)]
    pub max_freq: u16,

    #[arg(long, default_value_t = String::from("DISCRETE"))]
    pub display_mode: String,

    #[arg(long, default_value_t = 1024)]
    pub fft_size: u32,
}

pub struct App {
    pub edit_in_device: bool,
    pub in_devices: Vec<Device>,
    pub fft_engine: FFTEngine,
    pub audio_lock: Arc<Mutex<StreamOutput>>,
    pub freq_step: f64,
    pub args: Args,
    pub stream: Stream,
}

impl App {
    pub fn new() -> App {
        let args = Args::parse();
        let SAMPLE_RATE: u32 = args.sample_rate;
        let BINS: usize = args.bins;
        let S: f64 = args.smoothing_constant;
        let FPS: u8 = args.fps;
        let MIN_FREQ: u16 = args.min_freq;
        let MAX_FREQ: u16 = args.max_freq;
        let DISPLAY_MODE: &str = args.display_mode.as_str();
        let fft_size = args.fft_size;
        let freq_step = f64::from(SAMPLE_RATE) / f64::from(fft_size);

        let data_lock = Arc::new(Mutex::new(StreamOutput { data: vec![] }));
        let main_data_lock = data_lock.clone();
        let device = cpal::default_host()
            .input_devices()
            .unwrap()
            .find(|possible_device| possible_device.name().unwrap() == "BlackHole 2ch")
            .unwrap();
        let custom_config = cpal::StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(SAMPLE_RATE), // default sample rate 44100
            buffer_size: cpal::BufferSize::Fixed(fft_size), // default buffer size cpal::BufferSize::Default
        };

        let mut edit_in_device = false;
        let mut devices: Vec<(usize, String)> = cpal::default_host()
            .input_devices()
            .unwrap()
            .enumerate()
            .map(|(i, d)| (i, d.name().unwrap()))
            .collect();

        let mut stream = device
            .build_input_stream(
                &custom_config,
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
                None,
            )
            .unwrap();
        stream.play().unwrap();

        let mut fft_engine = FFTEngine::new(SAMPLE_RATE, BINS, S, WindowType::Blackman);

        App {
            edit_in_device,
            fft_engine,
            audio_lock: main_data_lock,
            in_devices: vec![],
            freq_step,
            args,
            stream,
        }
    }

    pub fn update_state(&mut self) {
        let data = match self.audio_lock.lock() {
            Ok(mut res) => res.data.clone(),
            _ => return,
        };

        self.fft_engine.set_src_buf(&data);

        self.fft_engine.apply_window();
        self.fft_engine.apply_fft();
    }

    pub fn normalize_db(&self, value: f64) -> f64 {
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
}
