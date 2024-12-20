mod app;
mod display;
mod fft;
mod inputs;
mod ui;

use crate::{
    app::App,
    fft::*,
    inputs::{events::Events, key::Key, InputEvent},
};

use std::{
    io,
    time::{Duration, Instant},
};

use cpal::traits::HostTrait;

use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    prelude::Backend,
    Terminal,
};
use ui::ui;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode().unwrap();
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    // restore terminal
    disable_raw_mode().unwrap();
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .unwrap();
    terminal.show_cursor().unwrap();

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    let mut start = Instant::now();
    let mut tickindex = 0usize;
    let mut ticksum = 0f32;
    let mut ticklist = [0f32; 100];

    let tick_rate = Duration::from_millis(1000 / u64::try_from(app.args.fps).unwrap());
    let events = Events::new(tick_rate);

    loop {
        terminal.draw(|f| ui(f, &app)).unwrap();
        let newtick = start.elapsed().as_secs_f32();
        app.args.fps =
            (1f32 / calc_avg_tick(&mut tickindex, &mut ticksum, &mut ticklist, newtick)) as u8;
        start = Instant::now();

        let result = match events.next().unwrap() {
            InputEvent::Input(key) => key,
            InputEvent::Tick => Key::Unknown,
        };

        if result.is_unknown() {
            app.update_state();
        }

        if result.is_right_arrow() {
            let windows = [
                WindowType::Hanning,
                WindowType::Hamming,
                WindowType::Blackman,
                WindowType::Nuttall,
            ];
            let new_idx = ((app.fft_engine.get_window() as usize) + 1) % 4;
            app.fft_engine.set_window(windows[new_idx].clone());
        }

        if result.is_left_arrow() {
            if !app.edit_in_device {
                app.in_devices = cpal::default_host().input_devices().unwrap().collect();
            }
            app.edit_in_device = !app.edit_in_device;
        }

        if result.is_down_arrow() {
            if app.edit_in_device {}
        }

        if result.is_exit() {
            break;
        }
    }

    Ok(())
}

fn calc_avg_tick(
    tickindex: &mut usize,
    ticksum: &mut f32,
    ticklist: &mut [f32],
    newtick: f32,
) -> f32 {
    *ticksum -= ticklist[*tickindex];
    *ticksum += newtick;
    ticklist[*tickindex] = newtick;
    *tickindex = (*tickindex + 1) % 100;

    *ticksum / (ticklist.len() as f32)
}
