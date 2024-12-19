use std::{ops::Add, sync::mpsc::channel};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

fn main() {
    let host = cpal::default_host();
    for onedevice in host.input_devices().unwrap() {
        println!("{}", onedevice.name().unwrap());
    }
}
