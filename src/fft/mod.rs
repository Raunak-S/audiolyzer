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
    fft_bins: Vec<Vec<f64>>,
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
            fft_bins: vec![vec![]; bins],
            processed_values: vec![0.; bins],
            sample_rate,
            smoothing_base,
            window_fn,
            logger: std::fs::File::create("txt/output.txt").unwrap(),
        }
    }

    pub fn push_samples(&mut self, samples: &[f32]) {
        if samples.is_empty() {
            return;
        }
        self.curr_data = samples.to_owned();
        self.logger
            .write_all(
                format!(
                    "Curr_data.len {:?}\n",
                    self.curr_data.len()
                )
                .as_bytes(),
            )
            .unwrap();
    }

    pub fn get_curr_data(&self) -> Vec<f32> {
        self.curr_data.clone()
    }

    pub fn get_window(&self) -> WindowType {
        self.window_fn.clone()
    }

    pub fn set_window(&mut self, window_fn: WindowType) {
        self.window_fn = window_fn;
    }

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
        let window = window_fn
            .map(|f| f as f32)
            .collect::<Vec<f32>>();

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
        let freq_step = f64::try_from(self.sample_rate).unwrap() / self.curr_data.len() as f64;

        self.logger
            .write_all(format!("spectrum.len {:?}\n", spectrum.len()).as_bytes())
            .unwrap();

        // B_i = ((f_i / f_max) ** (1 / gamma)) * B_max

        /*
         *
         * Map the calculated frequencies into specific bins
         *
         * */

        for bin in &mut self.fft_bins {
            bin.clear();
        }

        for val in spectrum.iter().enumerate() {
            //if val.0>spectrum.len()/2-1 {break;}
            if freq_step * val.0 as f64 >= (self.sample_rate / 2) as f64 {
                continue;
            }
            // let bin_len = self.fft_bins.len();
            // let f_i = freq_step * val.0 as f64;
            // let f_max = (self.sample_rate / 2) as f64;
            // let gamma = 2.;
            // let b_max = (bin_len as f64 - 1.);

            let insert_idx = match self.fft_bins.len() {
                10 => ((val.0 as f64*freq_step).round() / 2000.) as usize,
                _ => (val.0 as f64*freq_step).round() as usize,
            };
            if insert_idx >= self.fft_bins.len() {
                continue
            }
            self.fft_bins[insert_idx]
                .push(val.1.norm());
        }


        // B_i' = B_(i-1)' * s' + B_i * (1 - s')
        // s' = s ** (1 / R)
        // R = NUM_OF_SAMPLES / SAMPLE_RATE
        for bin in self.fft_bins.iter().enumerate() {
            let y_value_raw = if bin.1.len() != 0 {
                bin
                .1[0] / (self.curr_data.len() as f64)
            } else {
                0.
            };

            
            let y_value_final = self.prev_data[bin.0] * self.smoothing_base + y_value_raw * (1. - self.smoothing_base);
            self.prev_data[bin.0] = y_value_final;
            self.processed_values[bin.0] = self.normalize_db(self.linear_to_db(y_value_final)) * 10.;
        }
    }

    fn linear_to_db(&self, value: f64) -> f64 {
        if value == 0f64 {
            -1000f64
        } else {
            20f64 * value.log10()
        }
    }

    fn normalize_db(&self, value: f64) -> f64 {
        
        let max_val = -25f64;
        let min_val = -85f64;

        let normal_val = (value-min_val) / (max_val - min_val);

        if normal_val < 0. {
            0.
        } else if normal_val > 1. {
            1.
        } else {
            normal_val
        }
    }

    pub fn get_bins(&self) -> Vec<f64> {
        // remove the first value because that is the DC component of FFT and has no frequency information
        self.processed_values[1..].to_vec()
    }
}
