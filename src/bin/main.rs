use std::{io, thread, time::Duration, sync::{Arc, atomic::{AtomicUsize, Ordering}, Mutex}};

use cpal::{traits::{HostTrait, DeviceTrait, StreamTrait}, Device, SampleRate};
use crossterm::{terminal::{enable_raw_mode, EnterAlternateScreen, disable_raw_mode, LeaveAlternateScreen}, execute, event::{EnableMouseCapture, DisableMouseCapture}};

use tui::{backend::CrosstermBackend, Terminal, widgets::{Block, Borders, Dataset, GraphType, Chart, Axis, BarChart}, symbols, style::{Style, Color, Modifier}, text::Span};
use realfft::{RealFftPlanner, num_traits::Pow};

use std::sync::mpsc::channel;

use audiolyzer::audio_thread;

const BINS: usize = 30;

fn main() -> Result<(), io::Error> {

    let data_lock = Arc::new(Mutex::new(vec![0f32]));
    let main_data_lock = data_lock.clone();
    std::thread::spawn(move || {
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
    });
    
    enable_raw_mode().unwrap();
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    // // restore terminal
    // disable_raw_mode().unwrap();
    // execute!(
    //     terminal.backend_mut(),
    //     LeaveAlternateScreen,
    //     DisableMouseCapture
    // ).unwrap();
    // terminal.show_cursor().unwrap();

    // B_i' = B_(i-1)' * s' + B_i * (1 - s')

    let mut prev_data_set = vec![0f64; BINS];
    let s_prime = 0.5;

    loop {
    let data = match main_data_lock.lock() {
        Ok(res) => res.clone(),
        _ => continue,
    };
    let freq_step = 44100f64/data.len() as f64;
    let mut real_planner = RealFftPlanner::<f64>::new();

    // create a FFT
    let r2c = real_planner.plan_fft_forward(data.len());
    // make input and output vectors
    let mut spectrum = r2c.make_output_vec();

    let mut arr: Vec<f64> = data.iter().map(|val| *val as f64).collect();

    r2c.process(&mut arr[..], &mut spectrum).unwrap();
   
    let mut display_vec = vec![];

    let bars = 64;

    let mut bins: [Vec<f64>; BINS] = Default::default();

    // B_i = ((f_i / f_max) ** (1 / gamma)) * B_max

    for val in spectrum.iter().enumerate() {
        //if val.0>spectrum.len()/2-1 {break;}
        bins[((freq_step*val.0 as f64/(freq_step*spectrum.len() as f64)).powf(1./2.)*BINS as f64) as usize].push(val.1.norm_sqr());
    }

    for bin in bins.iter().enumerate() {
        display_vec.push(("1", 0u64.max((prev_data_set[bin.0]*s_prime+bin.1.iter().copied().fold(f64::NEG_INFINITY, f64::max).log2()*(1.-s_prime)) as u64)));
    }

    // loop {
    //     if counter > spectrum.len() { break; }
    //     let mut max = 0f64;
    //     for val in &spectrum[counter..((counter*2)+1).min(spectrum.len())] {
    //         max = max.max(val.norm_sqr());
    //     }
    //     counter = counter*2+1;
    //     display_vec.push(("1", 0u64.max((prev_data_set[i]*s_prime+ max.log2()*(1.-s_prime)) as u64)));
    //     i += 1;
    // }
    prev_data_set = display_vec.iter().map(|val| val.1 as f64).collect();

    let datasets = vec![
        Dataset::default()
            .name("sound")
            .marker(symbols::Marker::Dot)
            .graph_type(GraphType::Scatter)
            .style(Style::default().fg(Color::Cyan))
            // .data(&display_vec)
    ];
    let owned_bound = (bars+bars/8).to_string();
    let chart = Chart::new(datasets)
        .block(Block::default().title("Chart"))
        .x_axis(Axis::default()
            .title(Span::styled(data.len().to_string(), Style::default().fg(Color::Red)))
            .style(Style::default().fg(Color::White))
            .bounds([0.0, (30) as f64])
            .labels(["0.0", &owned_bound[..]].iter().cloned().map(Span::from).collect()))
        .y_axis(Axis::default()
            .title(Span::styled("Y Axis", Style::default().fg(Color::Red)))
            .style(Style::default().fg(Color::White))
            .bounds([0.0, 10.0])
            .labels(["-0.03", "0.0", "0.03"].iter().cloned().map(Span::from).collect()));

    let bar = BarChart::default()
    .block(Block::default().title(data.len().to_string()).borders(Borders::ALL))
    .bar_width(3)
    .bar_gap(1)
    .bar_style(Style::default().fg(Color::Yellow).bg(Color::Red))
    .value_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
    .label_style(Style::default().fg(Color::White))
    .data(&display_vec[..])
    .max(20);

    terminal.draw(|f| {
        let size = f.size();
        let block = Block::default()
            .title("Block")
            .borders(Borders::ALL);
        f.render_widget(bar, size);
    }).unwrap();

}

    Ok(())
}




