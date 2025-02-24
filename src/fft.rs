use std::io::Write;

use apodize::CosineWindowIter;

#[derive(Clone, Debug)]
pub enum WindowType {
    Hanning,
    Hamming,
    Blackman,
    Nuttall,
}

pub struct FFTEngine {
    prev_data: Vec<f64>,
    curr_data: Vec<f32>,
    sample_rate: u32,
    smoothing_base: f64,
    processed_values: Vec<f64>,
    window_fn: WindowType,
    logger: std::fs::File,
}

impl FFTEngine {
    pub fn new(sample_rate: u32, bins: usize, smoothing_base: f64, window_fn: WindowType) -> Self {
        FFTEngine {
            prev_data: vec![0.; bins],
            curr_data: vec![],
            processed_values: vec![-85.; bins],
            sample_rate,
            smoothing_base,
            window_fn,
            logger: std::fs::File::create("txt/output.txt").unwrap(),
        }
    }

    // GETTERS AND SETTERS

    pub fn set_window(&mut self, window_fn: WindowType) {
        self.window_fn = window_fn;
    }

    pub fn get_window(&self) -> WindowType {
        self.window_fn.clone()
    }

    pub fn set_src_buf(&mut self, samples: &[f32]) {
        if samples.is_empty() {
            return;
        }
        self.curr_data = samples.to_owned();
        self.log(format!("Curr_data.len {:?}\n", self.curr_data.len()));
    }

    pub fn get_src_buf(&self) -> Vec<f32> {
        self.curr_data.clone()
    }

    // READ-ONLY GETTERS

    pub fn get_bins(&self) -> Vec<f64> {
        // remove the first value because that is the DC component of FFT and has no frequency information
        self.processed_values.to_vec()
    }

    // HELPER FUNCTIONS

    pub fn apply_window(&mut self) {
        if !(1 < self.curr_data.len()) {
            return;
        }

        let window_fn: CosineWindowIter = match self.window_fn {
            WindowType::Hanning => apodize::hanning_iter(self.curr_data.len()),
            WindowType::Blackman => apodize::blackman_iter(self.curr_data.len()),
            WindowType::Hamming => apodize::hamming_iter(self.curr_data.len()),
            WindowType::Nuttall => apodize::nuttall_iter(self.curr_data.len()),
        };
        let window = window_fn.map(|f| f as f32).collect::<Vec<f32>>();

        self.curr_data = window
            .iter()
            .zip(self.curr_data.iter())
            .map(|f| f.0 * f.1)
            .collect();
    }

    pub fn apply_fft(&mut self) {
        let mut real_planner = realfft::RealFftPlanner::<f64>::new();
        let r2c = real_planner.plan_fft_forward(self.curr_data.len());
        // make input and output vectors
        let mut spectrum = r2c.make_output_vec();
        let mut arr: Vec<f64> = self.curr_data.iter().map(|val| *val as f64).collect();

        r2c.process(&mut arr[..], &mut spectrum).unwrap();

        self.log(format!("spectrum.len {:?}\n", spectrum.len()));

        let magnitudes = spectrum
            .iter()
            .map(|complex| complex.norm())
            .collect::<Vec<f64>>();

        for (i, magnitude) in magnitudes.iter().enumerate() {
            let y_value_raw = magnitude / (self.curr_data.len() as f64);

            self.prev_data[i] =
                self.prev_data[i] * self.smoothing_base + y_value_raw * (1. - self.smoothing_base);
            self.processed_values[i] = self.linear_to_db(self.prev_data[i]);
        }
    }

    fn linear_to_db(&self, value: f64) -> f64 {
        if value == 0f64 {
            -1000f64
        } else {
            20f64 * value.log10()
        }
    }

    pub fn log(&mut self, s: String) {
        self.logger.write_all(s.as_bytes()).unwrap();
    }
}
