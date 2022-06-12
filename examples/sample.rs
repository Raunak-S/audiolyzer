use std::sync::mpsc::channel;

use audiolyzer::audio_thread::get_audio_data;

fn main() {
    let (tx, rx) = channel();

    std::thread::spawn(|| get_audio_data(tx));

    loop {
        dbg!(rx.recv().unwrap().len());
    }

}