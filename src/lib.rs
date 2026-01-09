pub mod protocol;
pub mod modulation;
pub mod audio;
pub mod error;
pub mod codec;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

pub use protocol::*;
pub use modulation::*;
pub use audio::*;
pub use error::*;
pub use codec::*;

pub const SAMPLE_RATE: u32 = 48000;
pub const DEFAULT_SYMBOL_DURATION_MS: u32 = 50;
pub const NUM_TONES: usize = 16;
pub const WAKE_UP_FREQUENCY: f32 = 18500.0;
pub const WAKE_UP_DURATION_MS: u32 = 100;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransmissionMode {
    Audible,
    Ultrasonic,
}

impl TransmissionMode {
    pub fn base_frequency(&self) -> f32 {
        match self {
            TransmissionMode::Audible => 1000.0,
            TransmissionMode::Ultrasonic => 17000.0,
        }
    }

    pub fn frequency_step(&self) -> f32 {
        match self {
            TransmissionMode::Audible => 100.0,
            TransmissionMode::Ultrasonic => 150.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub mode: TransmissionMode,
    pub symbol_duration_ms: u32,
    pub sample_rate: u32,
    pub volume: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: TransmissionMode::Audible,
            symbol_duration_ms: DEFAULT_SYMBOL_DURATION_MS,
            sample_rate: SAMPLE_RATE,
            volume: 0.5,
        }
    }
}
