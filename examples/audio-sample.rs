use core::time;
use std::{
    sync::{Arc, Mutex},
    thread::sleep,
    time::Duration,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    InputStreamTimestamp,
};

pub struct Info {
    data: Vec<f32>,
    timestamp: Option<InputStreamTimestamp>,
}

fn main() {
    let main_data_lock = Arc::new(Mutex::new(Info {
        data: vec![],
        timestamp: None,
    }));
    let data_lock = main_data_lock.clone();
    let host = cpal::default_host();
    let device = host
        .devices()
        .unwrap()
        .find(|possible_device| possible_device.name().unwrap() == "BlackHole 2ch")
        .unwrap();
    let custom_config = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(44100), // default sample rate 44100
        buffer_size: cpal::BufferSize::Fixed(1024), // device default is cpal::BufferSize::Default
    };

    let stream = device
        .build_input_stream(
            &custom_config.into(),
            //&default_config.into(),
            move |data: &[f32], cb_info: &cpal::InputCallbackInfo| {
                // react to stream events and read or write stream data here.
                match data_lock.lock() {
                    Ok(mut info) => {
                        info.data = data.to_vec();
                        info.timestamp = Some(cb_info.timestamp());
                    }
                    Err(_) => {}
                }
            },
            move |err| {
                // react to errors here.
                eprintln!("{err}");
                panic!()
            },
        )
        .unwrap();
    stream.play().unwrap();

    let mut tmp = vec![];
    for _ in 0..10 {
        sleep(Duration::from_millis(1000 / 60));
        let info = main_data_lock.lock().unwrap();
        tmp.push((info.timestamp.unwrap(), info.data.len()));
    }
    for timestamp in tmp {
        println!("{:?}\n{:?}", timestamp.0, timestamp.1);
    }
}
