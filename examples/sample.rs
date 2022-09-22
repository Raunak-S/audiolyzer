use std::{ops::Add, sync::mpsc::channel};

use audiolyzer::audio_thread::get_audio_data;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

fn main() {
    let host = cpal::default_host();
    let device = host.default_input_device().unwrap();

    let custom_config = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(44100),
        buffer_size: cpal::BufferSize::Default,
    };
    let counter = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let moved_counter = std::sync::Arc::clone(&counter);
    let stream = device
        .build_input_stream(
            &custom_config.into(),
            move |data: &[f32], inputcallback: &cpal::InputCallbackInfo| {
                // react to stream events and read or write stream data here.
                // dbg!({inputcallback});
                moved_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            },
            move |err| {
                // react to errors here.
                eprintln!("{err}");
                panic!()
            },
        )
        .unwrap();
    stream.play().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(2000));
    dbg!(counter);
}
