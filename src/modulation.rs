use crate::{Config, TransmissionMode, SAMPLE_RATE, WAKE_UP_DURATION_MS, WAKE_UP_FREQUENCY};
use rustfft::{num_complex::Complex, FftPlanner};
use std::f32::consts::PI;

pub struct MFSKModulator {
    config: Config,
    frequencies: Vec<f32>,
}

impl MFSKModulator {
    pub fn new(config: Config) -> Self {
        let base_freq = config.mode.base_frequency();
        let step = config.mode.frequency_step();

        let frequencies: Vec<f32> = (0..16).map(|i| base_freq + (i as f32) * step).collect();

        Self { config, frequencies }
    }

    pub fn generate_tone(&self, frequency: f32, duration_ms: u32) -> Vec<f32> {
        let num_samples = (self.config.sample_rate as f32 * duration_ms as f32 / 1000.0) as usize;
        let mut samples = Vec::with_capacity(num_samples);

        for i in 0..num_samples {
            let t = i as f32 / self.config.sample_rate as f32;
            let sample = (2.0 * PI * frequency * t).sin() * self.config.volume;

            let fade_samples = (self.config.sample_rate as f32 * 0.005) as usize;
            let fade = if i < fade_samples {
                i as f32 / fade_samples as f32
            } else if i > num_samples - fade_samples {
                (num_samples - i) as f32 / fade_samples as f32
            } else {
                1.0
            };

            samples.push(sample * fade);
        }

        samples
    }

    pub fn generate_wake_up_tone(&self) -> Vec<f32> {
        self.generate_tone(WAKE_UP_FREQUENCY, WAKE_UP_DURATION_MS)
    }

    pub fn modulate(&self, data: &[u8]) -> Vec<f32> {
        let mut samples = Vec::new();

        samples.extend(self.generate_wake_up_tone());

        let silence_samples = (self.config.sample_rate as f32 * 0.02) as usize;
        samples.extend(vec![0.0f32; silence_samples]);

        for byte in data {
            let high_nibble = (byte >> 4) & 0x0F;
            let low_nibble = byte & 0x0F;

            let high_freq = self.frequencies[high_nibble as usize];
            let low_freq = self.frequencies[low_nibble as usize];

            samples.extend(self.generate_tone(high_freq, self.config.symbol_duration_ms));
            samples.extend(self.generate_tone(low_freq, self.config.symbol_duration_ms));
        }

        samples.extend(self.generate_wake_up_tone());

        samples
    }

    pub fn get_frequencies(&self) -> &[f32] {
        &self.frequencies
    }
}

pub struct MFSKDemodulator {
    config: Config,
    frequencies: Vec<f32>,
    fft_planner: FftPlanner<f32>,
}

impl MFSKDemodulator {
    pub fn new(config: Config) -> Self {
        let base_freq = config.mode.base_frequency();
        let step = config.mode.frequency_step();

        let frequencies: Vec<f32> = (0..16).map(|i| base_freq + (i as f32) * step).collect();

        Self {
            config,
            frequencies,
            fft_planner: FftPlanner::new(),
        }
    }

    pub fn goertzel(&self, samples: &[f32], target_freq: f32) -> f32 {
        let n = samples.len();
        let k = (target_freq * n as f32 / self.config.sample_rate as f32).round() as usize;
        let omega = 2.0 * PI * k as f32 / n as f32;
        let coeff = 2.0 * omega.cos();

        let mut s0 = 0.0f32;
        let mut s1 = 0.0f32;
        let mut s2 = 0.0f32;

        for &sample in samples {
            s0 = sample + coeff * s1 - s2;
            s2 = s1;
            s1 = s0;
        }

        let power = s1 * s1 + s2 * s2 - s1 * s2 * coeff;
        power.sqrt()
    }

    pub fn detect_wake_up(&self, samples: &[f32]) -> Option<usize> {
        let window_size = (self.config.sample_rate as f32 * WAKE_UP_DURATION_MS as f32 / 1000.0 / 2.0) as usize;
        let step = window_size / 4;

        for i in (0..samples.len().saturating_sub(window_size)).step_by(step) {
            let window = &samples[i..i + window_size];
            let wake_mag = self.goertzel(window, WAKE_UP_FREQUENCY);

            let data_mag: f32 = self.frequencies.iter()
                .map(|&f| self.goertzel(window, f))
                .fold(0.0f32, |a, b| a.max(b));

            if wake_mag > 0.01 && wake_mag > data_mag * 1.5 {
                let wake_end = i + (self.config.sample_rate as f32 * WAKE_UP_DURATION_MS as f32 / 1000.0) as usize;
                return Some(wake_end);
            }
        }

        None
    }

    pub fn detect_symbol(&self, samples: &[f32]) -> u8 {
        let mut max_magnitude = 0.0f32;
        let mut detected_index = 0u8;

        for (i, &freq) in self.frequencies.iter().enumerate() {
            let magnitude = self.goertzel(samples, freq);
            if magnitude > max_magnitude {
                max_magnitude = magnitude;
                detected_index = i as u8;
            }
        }

        detected_index
    }

    pub fn demodulate(&mut self, samples: &[f32]) -> Option<Vec<u8>> {
        let start_pos = self.detect_wake_up(samples)?;

        let symbol_samples = (self.config.sample_rate as f32 * self.config.symbol_duration_ms as f32 / 1000.0) as usize;
        let mut pos = start_pos + (self.config.sample_rate as f32 * 0.02) as usize;

        let mut data = Vec::new();
        let mut nibbles = Vec::new();

        while pos + symbol_samples <= samples.len() {
            let window = &samples[pos..pos + symbol_samples];

            let wake_mag = self.goertzel(window, WAKE_UP_FREQUENCY);
            let data_mag: f32 = self.frequencies.iter()
                .map(|&f| self.goertzel(window, f))
                .fold(0.0f32, |a, b| a.max(b));

            if wake_mag > data_mag * 1.5 && wake_mag > 0.01 {
                break;
            }

            let symbol = self.detect_symbol(window);
            nibbles.push(symbol);

            pos += symbol_samples;
        }

        for chunk in nibbles.chunks(2) {
            if chunk.len() == 2 {
                let byte = (chunk[0] << 4) | (chunk[1] & 0x0F);
                data.push(byte);
            }
        }

        if data.is_empty() {
            None
        } else {
            Some(data)
        }
    }

    pub fn analyze_spectrum(&mut self, samples: &[f32]) -> Vec<(f32, f32)> {
        let fft_size = 4096;
        let fft = self.fft_planner.plan_fft_forward(fft_size);

        let mut input: Vec<Complex<f32>> = samples
            .iter()
            .take(fft_size)
            .map(|&s| Complex::new(s, 0.0))
            .collect();

        while input.len() < fft_size {
            input.push(Complex::new(0.0, 0.0));
        }

        fft.process(&mut input);

        let freq_resolution = self.config.sample_rate as f32 / fft_size as f32;

        input
            .iter()
            .take(fft_size / 2)
            .enumerate()
            .map(|(i, c)| {
                let freq = i as f32 * freq_resolution;
                let magnitude = (c.re * c.re + c.im * c.im).sqrt() / fft_size as f32;
                (freq, magnitude)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modulation_roundtrip() {
        let config = Config::default();
        let modulator = MFSKModulator::new(config.clone());
        let mut demodulator = MFSKDemodulator::new(config);

        let data = vec![0xAB, 0xCD, 0x12, 0x34];
        let samples = modulator.modulate(&data);
        let decoded = demodulator.demodulate(&samples);

        assert!(decoded.is_some());
        assert_eq!(decoded.unwrap(), data);
    }

    #[test]
    fn test_goertzel() {
        let config = Config::default();
        let demodulator = MFSKDemodulator::new(config.clone());

        let freq = 1000.0;
        let samples: Vec<f32> = (0..4800)
            .map(|i| (2.0 * PI * freq * i as f32 / SAMPLE_RATE as f32).sin())
            .collect();

        let magnitude = demodulator.goertzel(&samples, freq);
        assert!(magnitude > 0.1);

        let other_magnitude = demodulator.goertzel(&samples, 2000.0);
        assert!(magnitude > other_magnitude * 5.0);
    }
}
