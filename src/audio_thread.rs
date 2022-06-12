use std::{io, thread, time::Duration, sync::{Arc, atomic::{AtomicUsize, Ordering}}};

use cpal::{traits::{HostTrait, DeviceTrait, StreamTrait}, Device, SampleRate, Host};
use crossterm::{terminal::{enable_raw_mode, EnterAlternateScreen, disable_raw_mode, LeaveAlternateScreen}, execute, event::{EnableMouseCapture, DisableMouseCapture}};

use tui::{backend::CrosstermBackend, Terminal, widgets::{Block, Borders, Dataset, GraphType, Chart, Axis, BarChart}, symbols, style::{Style, Color, Modifier}, text::Span};
use realfft::RealFftPlanner;

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

pub fn get_audio_data() -> fn() -> i32 {
    || {
        3
    }
}
