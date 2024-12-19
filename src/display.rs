use ratatui::{
    style::Color,
    widgets::canvas::{Context, Line, Points},
};

pub trait DisplayStrategy {
    fn render(&self, ctx: &mut Context, bins: &Vec<f64>, freq_step: f64);
}

pub struct DiscreteStrategy;

// creates display data vector of lines for the canvas widget
impl DisplayStrategy for DiscreteStrategy {
    fn render(&self, ctx: &mut Context, bins: &Vec<f64>, freq_step: f64) {
        let mut display_vec = vec![];
        for bin in bins.iter().enumerate() {
            if *bin.1 != 0f64 {
                display_vec.push(Line {
                    x1: freq_step * bin.0 as f64,
                    y1: 0.0,
                    x2: freq_step * bin.0 as f64,
                    y2: *bin.1,
                    color: Color::White,
                });
            }
        }

        for line in &display_vec {
            ctx.draw(line);
        }
    }
}

pub struct PointStrategy;

// creates display data vector of points for the canvas widget
impl DisplayStrategy for PointStrategy {
    fn render(&self, ctx: &mut Context, bins: &Vec<f64>, freq_step: f64) {
        let mut display_vec = vec![];
        for bin in bins.iter().enumerate() {
            if *bin.1 != 0f64 {
                display_vec.push((freq_step * bin.0 as f64, *bin.1));
            }
        }

        ctx.draw(&Points {
            coords: &display_vec,
            color: Color::White,
        });
    }
}

pub struct LineStrategy;

// creates display data vector of lines as an area graph for the canvas widget
impl DisplayStrategy for LineStrategy {
    fn render(&self, ctx: &mut Context, bins: &Vec<f64>, freq_step: f64) {
        let mut display_vec: Vec<Line> = vec![];
        let mut l = 0usize;
        for idx in 1..bins.len() {
            if bins[idx] != 0f64 {
                display_vec.push(Line {
                    x1: freq_step * l as f64,
                    y1: bins[l],
                    x2: freq_step * idx as f64,
                    y2: bins[idx],
                    color: Color::White,
                });
                l = idx;
            }
        }
        display_vec.push(Line {
            x1: freq_step * l as f64,
            y1: 0f64,
            x2: freq_step * l as f64,
            y2: bins[l],
            color: Color::White,
        });

        for line in &display_vec {
            ctx.draw(line);
        }
    }
}

// // creates display data vector for the bar widget

// fn create_bar_data(bins: Vec<f64>) -> Vec<(String, u64)> {
//     // freq_ranges holds strings of tui labels so that they are not
//     // dropped before being used in the bar chart display
//     let mut display_vec = vec![];
//     let freq_ranges: Vec<String> = (0..bins.len()).map(|label| label.to_string()).collect();
//     for bin in bins.iter().enumerate() {
//         display_vec.push((freq_ranges[bin.0].clone(), *bin.1 as u64));
//     }
//     display_vec
// }

pub struct DisplayStrategyFactory;

impl DisplayStrategyFactory {
    pub fn get_display_strategy(strategy: &str) -> Box<dyn DisplayStrategy> {
        match strategy {
            "DISCRETE" => Box::new(DiscreteStrategy),
            "POINT" => Box::new(PointStrategy),
            "LINE" => Box::new(LineStrategy),
            _ => Box::new(DiscreteStrategy),
        }
    }
}
