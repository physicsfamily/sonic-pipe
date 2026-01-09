use crate::error::{Result, SonicPipeError};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, StreamConfig};
use std::sync::{Arc, Mutex};

pub struct AudioOutput {
    device: Device,
    config: StreamConfig,
}

impl AudioOutput {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| SonicPipeError::AudioDevice("No output device found".into()))?;

        let supported_config = device
            .default_output_config()
            .map_err(|e| SonicPipeError::AudioDevice(e.to_string()))?;

        let config = StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(48000),
            buffer_size: cpal::BufferSize::Default,
        };

        Ok(Self { device, config })
    }

    pub fn play_samples(&self, samples: Vec<f32>) -> Result<()> {
        let samples = Arc::new(Mutex::new(samples));
        let position = Arc::new(Mutex::new(0usize));
        let finished = Arc::new(Mutex::new(false));

        let samples_clone = Arc::clone(&samples);
        let position_clone = Arc::clone(&position);
        let finished_clone = Arc::clone(&finished);

        let stream = self
            .device
            .build_output_stream(
                &self.config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let samples = samples_clone.lock().unwrap();
                    let mut pos = position_clone.lock().unwrap();

                    for sample in data.iter_mut() {
                        if *pos < samples.len() {
                            *sample = samples[*pos];
                            *pos += 1;
                        } else {
                            *sample = 0.0;
                            *finished_clone.lock().unwrap() = true;
                        }
                    }
                },
                |err| eprintln!("Audio output error: {}", err),
                None,
            )
            .map_err(|e| SonicPipeError::AudioDevice(e.to_string()))?;

        stream
            .play()
            .map_err(|e| SonicPipeError::AudioDevice(e.to_string()))?;

        loop {
            std::thread::sleep(std::time::Duration::from_millis(10));
            if *finished.lock().unwrap() {
                break;
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(100));

        Ok(())
    }
}

pub struct AudioInput {
    device: Device,
    config: StreamConfig,
}

impl AudioInput {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| SonicPipeError::AudioDevice("No input device found".into()))?;

        let config = StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(48000),
            buffer_size: cpal::BufferSize::Default,
        };

        Ok(Self { device, config })
    }

    pub fn record_samples(&self, duration_ms: u32) -> Result<Vec<f32>> {
        let num_samples = (48000.0 * duration_ms as f32 / 1000.0) as usize;
        let samples = Arc::new(Mutex::new(Vec::with_capacity(num_samples)));
        let samples_clone = Arc::clone(&samples);

        let stream = self
            .device
            .build_input_stream(
                &self.config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let mut samples = samples_clone.lock().unwrap();
                    samples.extend_from_slice(data);
                },
                |err| eprintln!("Audio input error: {}", err),
                None,
            )
            .map_err(|e| SonicPipeError::AudioDevice(e.to_string()))?;

        stream
            .play()
            .map_err(|e| SonicPipeError::AudioDevice(e.to_string()))?;

        std::thread::sleep(std::time::Duration::from_millis(duration_ms as u64));

        drop(stream);

        let result = samples.lock().unwrap().clone();
        Ok(result)
    }

    pub fn record_until_complete<F>(&self, mut check_fn: F, timeout_ms: u32) -> Result<Vec<f32>>
    where
        F: FnMut(&[f32]) -> bool,
    {
        let samples = Arc::new(Mutex::new(Vec::new()));
        let samples_clone = Arc::clone(&samples);

        let stream = self
            .device
            .build_input_stream(
                &self.config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let mut samples = samples_clone.lock().unwrap();
                    samples.extend_from_slice(data);
                },
                |err| eprintln!("Audio input error: {}", err),
                None,
            )
            .map_err(|e| SonicPipeError::AudioDevice(e.to_string()))?;

        stream
            .play()
            .map_err(|e| SonicPipeError::AudioDevice(e.to_string()))?;

        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms as u64);

        loop {
            std::thread::sleep(std::time::Duration::from_millis(50));

            let current_samples = samples.lock().unwrap().clone();
            if check_fn(&current_samples) {
                break;
            }

            if start.elapsed() > timeout {
                return Err(SonicPipeError::Timeout);
            }
        }

        drop(stream);

        let result = samples.lock().unwrap().clone();
        Ok(result)
    }
}

pub fn list_audio_devices() -> Vec<String> {
    let host = cpal::default_host();
    let mut devices = Vec::new();

    if let Ok(output_devices) = host.output_devices() {
        for device in output_devices {
            if let Ok(name) = device.name() {
                devices.push(format!("Output: {}", name));
            }
        }
    }

    if let Ok(input_devices) = host.input_devices() {
        for device in input_devices {
            if let Ok(name) = device.name() {
                devices.push(format!("Input: {}", name));
            }
        }
    }

    devices
}
