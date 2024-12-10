use std::{ops::Add, sync::mpsc::channel};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

fn main() {
    let host = cpal::default_host();
    for onedevice in host.devices().unwrap() {
        println!("{}", onedevice.name().unwrap());
    }
    let device = host
        .devices()
        .unwrap()
        .find(|possible_device| possible_device.name().unwrap() == "Multi-Output Device")
        .unwrap();
    println!("{}", device.name().unwrap());
    return;
    let custom_config = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(44100),
        buffer_size: cpal::BufferSize::Default,
    };
    let stream = device
        .build_input_stream(
            &custom_config.into(),
            move |data: &[f32], inputcallback: &cpal::InputCallbackInfo| {
                // react to stream events and read or write stream data here.
                // dbg!({inputcallback});
            },
            move |err| {
                // react to errors here.
                eprintln!("{err}");
                panic!()
            },
        )
        .unwrap();
    stream.play().unwrap();
}
