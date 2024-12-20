use crate::{
    app::App,
    display::DisplayStrategyFactory,
};


use cpal::traits::DeviceTrait;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{
        canvas::Canvas, Block, Borders, Clear, Paragraph,
    },
    Frame,
};

pub struct UI;

pub fn ui(f: &mut Frame, app: &App) {
    let canvas = Canvas::default()
        .block(
            Block::default()
                .title(format!(
                    "audiolyzer - Window: {:?} - FPS: {:?}",
                    app.fft_engine.get_window(),
                    app.args.fps
                ))
                .borders(Borders::ALL),
        )
        .x_bounds([app.args.min_freq.into(), app.args.max_freq.into()])
        .y_bounds([0.0, 1.0])
        .paint(|ctx| {
            let mut freq_data = app.fft_engine.get_bins();
            freq_data.iter_mut().for_each(|x| *x = app.normalize_db(*x));
            let s = DisplayStrategyFactory::get_display_strategy(&app.args.display_mode);
            s.render(ctx, &freq_data, app.freq_step);
        });

    let size = f.area();

    match app.edit_in_device {
        true => {
            let popup_block = Block::default()
                .title("")
                .borders(Borders::NONE)
                .style(Style::default().bg(Color::DarkGray));
            let area = centered_rect(60, 25, size);

            let popup_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(Constraint::from_lengths(vec![1; app.in_devices.len()]))
                .split(area);

            f.render_widget(canvas, size);
            f.render_widget(Clear, area);
            f.render_widget(popup_block, area);
            app.in_devices.iter().enumerate().for_each(|(i, d)| {
                let text = Paragraph::new(d.name().unwrap().clone());
                f.render_widget(text, popup_chunks[i]);
            });
        }
        false => f.render_widget(canvas, size),
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    // Cut the given rectangle into three vertical pieces
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    // Then cut the middle vertical piece into three width-wise pieces
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1] // Return the middle chunk
}
