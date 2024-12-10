use std::io::Write;

pub struct FFTEngine {
    prev_data: Vec<f64>,
    curr_data: Vec<f32>,
    fft_bins: Vec<Vec<f64>>,
    sample_rate: u32,
    smoothing_base: f64,
    processed_values: Vec<f64>,
    logger: std::fs::File,
}

impl FFTEngine {
    pub fn new(sample_rate: u32, bins: usize, smoothing_base: f64) -> Self {
        FFTEngine {
            prev_data: vec![0.; bins],
            curr_data: vec![],
            fft_bins: vec![vec![]; bins],
            sample_rate,
            smoothing_base,
            processed_values: vec![0.; bins],
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
                    "Curr_data.len {:?}\nCurr_data {:?}\n",
                    self.curr_data.len(),
                    self.curr_data.to_owned()
                )
                .as_bytes(),
            )
            .unwrap();
    }

    pub fn get_curr_data(&self) -> Vec<f32> {
        self.curr_data.clone()
    }

    pub fn apply_hanning_window(&mut self) {
        if !(1 < self.curr_data.len()) {
            return;
        }
        let window = apodize::hanning_iter(self.curr_data.len())
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
            .write_all(format!("spectrum {:?}\nspectrum.len {:?}\nprocessed_values {:?}\n", spectrum, spectrum.len(), self.processed_values,).as_bytes())
            .unwrap();

        // B_i = ((f_i / f_max) ** (1 / gamma)) * B_max

        /*
         *
         * Map the calculated frequencies into specific bins
         *
         * */

        let smoothing = self.smoothing_base.powf(1. / freq_step);

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
                .push(val.1.norm_sqr());
        }

        // B_i' = B_(i-1)' * s' + B_i * (1 - s')
        // s' = s ** (1 / R)
        // R = NUM_OF_SAMPLES / SAMPLE_RATE
        for bin in self.fft_bins.iter().enumerate() {
            let y_value_raw = bin
                .1
                .iter()
                .copied()
                .fold(1., f64::max)
                .log10();
            let y_value_final = if y_value_raw > self.prev_data[bin.0] {
                y_value_raw
            } else {
                self.prev_data[bin.0] * smoothing + y_value_raw * (1. - smoothing)
            };
            self.processed_values[bin.0] = y_value_final * 20.;
            self.prev_data[bin.0] = y_value_final;
        }
    }

    pub fn get_bins(&self) -> Vec<f64> {
        // remove the first value because that is the DC component of FFT and has no frequency information
        self.processed_values[1..].to_vec()
    }
}
