use std::{io, thread, time::Duration, sync::{Arc, atomic::{AtomicUsize, Ordering}, mpsc::Sender, Mutex}};

use cpal::{traits::{HostTrait, DeviceTrait, StreamTrait}, Device, SampleRate, Host};
use crossterm::{terminal::{enable_raw_mode, EnterAlternateScreen, disable_raw_mode, LeaveAlternateScreen}, execute, event::{EnableMouseCapture, DisableMouseCapture}};

use tui::{backend::CrosstermBackend, Terminal, widgets::{Block, Borders, Dataset, GraphType, Chart, Axis, BarChart}, symbols, style::{Style, Color, Modifier}, text::Span};
use realfft::RealFftPlanner;

use std::sync::mpsc::channel;

pub fn host_output_devices() {
    let host = cpal::default_host();
    let devices = host.input_devices().unwrap();
    for dev in devices {
        println!("---{}---", dev.name().unwrap());
    }
}

pub fn get_device(name: &str) -> Device {
    let host = cpal::default_host();
    for dev in host.input_devices().unwrap() {
        if dev.name().unwrap() == "default" {
            return dev;
        }
    }
    todo!()
}

pub fn get_audio_data(data_lock: Arc<Mutex<Vec<f32>>>) {

        let host = cpal::default_host();
        let device = host.default_input_device().unwrap();

        let custom_config = cpal::StreamConfig {
            channels : 1,
            sample_rate: cpal::SampleRate(44100),
            buffer_size: cpal::BufferSize::Default
        };

        let stream = device.build_input_stream(
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
        ).unwrap();
        stream.play().unwrap();
        loop {}
}
