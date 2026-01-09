#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::{
    codec::{compress, decompress, ReedSolomonCodec},
    modulation::{MFSKDemodulator, MFSKModulator},
    protocol::Packet,
    Config, TransmissionMode,
};

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct SonicPipeWasm {
    config: Config,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl SonicPipeWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(ultrasonic: bool) -> Self {
        console_error_panic_hook::set_once();

        Self {
            config: Config {
                mode: if ultrasonic {
                    TransmissionMode::Ultrasonic
                } else {
                    TransmissionMode::Audible
                },
                ..Default::default()
            },
        }
    }

    #[wasm_bindgen]
    pub fn set_symbol_duration(&mut self, duration_ms: u32) {
        self.config.symbol_duration_ms = duration_ms;
    }

    #[wasm_bindgen]
    pub fn set_volume(&mut self, volume: f32) {
        self.config.volume = volume.clamp(0.0, 1.0);
    }

    #[wasm_bindgen]
    pub fn encode(&self, data: &[u8]) -> Result<Vec<f32>, JsValue> {
        let compressed = compress(data);

        let ecc = ReedSolomonCodec::new()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let encoded = ecc
            .encode(&compressed)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let packet = Packet::new(encoded)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let packet_data = packet.serialize();

        let modulator = MFSKModulator::new(self.config.clone());
        let samples = modulator.modulate(&packet_data);

        Ok(samples)
    }

    #[wasm_bindgen]
    pub fn encode_string(&self, text: &str) -> Result<Vec<f32>, JsValue> {
        self.encode(text.as_bytes())
    }

    #[wasm_bindgen]
    pub fn decode(&self, samples: &[f32]) -> Result<Vec<u8>, JsValue> {
        let mut demodulator = MFSKDemodulator::new(self.config.clone());

        let raw_data = demodulator
            .demodulate(samples)
            .ok_or_else(|| JsValue::from_str("Failed to demodulate signal"))?;

        let packet = Packet::deserialize(&raw_data)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let ecc = ReedSolomonCodec::new()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let decoded = ecc
            .decode(&packet.payload)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let decompressed = decompress(&decoded)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(decompressed)
    }

    #[wasm_bindgen]
    pub fn decode_to_string(&self, samples: &[f32]) -> Result<String, JsValue> {
        let data = self.decode(samples)?;
        String::from_utf8(data)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen]
    pub fn get_frequencies(&self) -> Vec<f32> {
        let base_freq = self.config.mode.base_frequency();
        let step = self.config.mode.frequency_step();
        (0..16).map(|i| base_freq + (i as f32) * step).collect()
    }

    #[wasm_bindgen]
    pub fn get_sample_rate(&self) -> u32 {
        self.config.sample_rate
    }

    #[wasm_bindgen]
    pub fn get_symbol_duration_samples(&self) -> u32 {
        (self.config.sample_rate as f32 * self.config.symbol_duration_ms as f32 / 1000.0) as u32
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn init() {
    console_error_panic_hook::set_once();
}
