use std::{io, thread, time::Duration};

use cpal::{traits::{HostTrait, DeviceTrait, StreamTrait}, Device, SampleRate};
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

fn main() -> Result<(), io::Error> {
    //host_output_devices();
    //panic!();
    // setup terminal    
    let host = cpal::default_host();
    let device = get_device("");
    let mut supported_configs_range = device.supported_input_configs()
        .expect("error while querying configs");

    let supported_config = supported_configs_range.next()
    .expect("no supported config?!")
    .with_sample_rate(SampleRate(44100));


    // enable_raw_mode().unwrap();
    // let mut stdout = io::stdout();
    // execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
    // let backend = CrosstermBackend::new(stdout);
    // let mut terminal = Terminal::new(backend).unwrap();

    //panic!();

    // restore terminal
    // disable_raw_mode().unwrap();
    // execute!(
    //     terminal.backend_mut(),
    //     LeaveAlternateScreen,
    //     DisableMouseCapture
    // ).unwrap();
    // terminal.show_cursor().unwrap();

    let stream = device.build_input_stream(
        &supported_config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            // react to stream events and read or write stream data here.
            {
                let mut data_vec = vec![];
                let mut bar_vec = vec![];
                let mut chart_vec = vec![];
                for i in (0..data.len()).step_by(2) {
                    if i==data.len()-1 {continue;}
                    data_vec.push(((data[i]+data[i+1])/2.) as f64);
                    chart_vec.push((i as f64/2., ((data[i]+data[i+1])/2.) as f64));
                    bar_vec.push(("", ((data[i]+data[i+1])/2.) as u64));
                }

                dbg!(data.len());
                panic!();

                let mut real_planner = RealFftPlanner::<f64>::new();

                // create a FFT
                let r2c = real_planner.plan_fft_forward(data_vec.len());
                // make input and output vectors
                let mut indata = r2c.make_input_vec();
                let mut spectrum = r2c.make_output_vec();


                r2c.process(&mut data_vec, &mut spectrum).unwrap();

                let complexvec = spectrum.clone();
                // let mut planner = FftPlanner::new();
                // let fft = planner.plan_fft_forward(complexvec.len());
                // fft.process(&mut complexvec);
                // dbg!(&complexvec);
                // dbg!(complexvec.len());
                // panic!();
                let display_vec: Vec<(f64,f64)> = (0..complexvec.len()).map(|i| (i as f64, (complexvec[i].im).atan2(complexvec[i].re))).collect();
                dbg!(display_vec);
                panic!();
                let datasets = vec![
                    Dataset::default()
                        .name("sound")
                        .marker(symbols::Marker::Dot)
                        .graph_type(GraphType::Scatter)
                        .style(Style::default().fg(Color::Cyan))
                        .data(&display_vec[..])
                ];

                let chart = Chart::new(datasets)
                    .block(Block::default().title("Chart"))
                    .x_axis(Axis::default()
                        .title(Span::styled("X Axis", Style::default().fg(Color::Red)))
                        .style(Style::default().fg(Color::White))
                        .bounds([0.0, 3100.0])
                        .labels(["0.0", "3000.0", "6100.0"].iter().cloned().map(Span::from).collect()))
                    .y_axis(Axis::default()
                        .title(Span::styled("Y Axis", Style::default().fg(Color::Red)))
                        .style(Style::default().fg(Color::White))
                        .bounds([-0.9, 0.9])
                        .labels(["-0.03", "0.0", "0.03"].iter().cloned().map(Span::from).collect()));

                let bar = BarChart::default()
                    .block(Block::default().title("BarChart").borders(Borders::ALL))
                    .bar_width(1)
                    .bar_gap(1)
                    .bar_style(Style::default().fg(Color::Yellow).bg(Color::Red))
                    .value_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
                    .label_style(Style::default().fg(Color::White))
                    .data(&bar_vec[..])
                    .max(4);

                // terminal.draw(|f| {
                //     let size = f.size();
                //     let block = Block::default()
                //         .title("Block")
                //         .borders(Borders::ALL);
                //     f.render_widget(chart, size);
                // }).unwrap();
            
                // thread::sleep(Duration::from_millis(5000));
            

            }
            // panic!()
        },
        move |err| {
            // react to errors here.
            eprintln!("{err}");
            panic!()
        },
    ).unwrap();
    stream.play().unwrap();
    loop{}

    Ok(())
}
