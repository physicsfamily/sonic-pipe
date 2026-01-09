use anyhow::Result;
use clap::{Parser, Subcommand};
use sonic_pipe_core::{
    audio::{AudioInput, AudioOutput},
    codec::{compress, decompress, ReedSolomonCodec},
    modulation::{MFSKDemodulator, MFSKModulator},
    protocol::Packet,
    Config, TransmissionMode, WAKE_UP_FREQUENCY,
};
use std::io::{self, Read, Write};

#[derive(Parser)]
#[command(name = "sonic-pipe")]
#[command(about = "Acoustic modem for air-gapped data transfer", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Send data via audio
    Send {
        /// Use ultrasonic mode (17-20kHz, semi-silent)
        #[arg(long, short)]
        ultrasonic: bool,

        /// Symbol duration in milliseconds
        #[arg(long, default_value = "50")]
        symbol_duration: u32,

        /// Volume level (0.0 - 1.0)
        #[arg(long, default_value = "0.5")]
        volume: f32,

        /// Data to send (if not provided, reads from stdin)
        #[arg(short, long)]
        data: Option<String>,
    },

    /// Receive data via audio
    Receive {
        /// Use ultrasonic mode (17-20kHz, semi-silent)
        #[arg(long, short)]
        ultrasonic: bool,

        /// Symbol duration in milliseconds
        #[arg(long, default_value = "50")]
        symbol_duration: u32,

        /// Timeout in seconds
        #[arg(long, default_value = "30")]
        timeout: u32,
    },

    /// List available audio devices
    Devices,

    /// Test audio transmission (loopback test)
    Test {
        /// Test message
        #[arg(default_value = "Hello, Sonic-Pipe!")]
        message: String,
    },
}

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Send {
            ultrasonic,
            symbol_duration,
            volume,
            data,
        } => {
            let input_data = match data {
                Some(d) => d.into_bytes(),
                None => {
                    let mut buffer = Vec::new();
                    io::stdin().read_to_end(&mut buffer)?;
                    buffer
                }
            };

            if input_data.is_empty() {
                eprintln!("Error: No data to send");
                std::process::exit(1);
            }

            let config = Config {
                mode: if ultrasonic {
                    TransmissionMode::Ultrasonic
                } else {
                    TransmissionMode::Audible
                },
                symbol_duration_ms: symbol_duration,
                volume,
                ..Default::default()
            };

            send_data(&input_data, &config)?;
        }

        Commands::Receive {
            ultrasonic,
            symbol_duration,
            timeout,
        } => {
            let config = Config {
                mode: if ultrasonic {
                    TransmissionMode::Ultrasonic
                } else {
                    TransmissionMode::Audible
                },
                symbol_duration_ms: symbol_duration,
                ..Default::default()
            };

            let data = receive_data(&config, timeout)?;
            io::stdout().write_all(&data)?;
            io::stdout().flush()?;
        }

        Commands::Devices => {
            let devices = sonic_pipe_core::audio::list_audio_devices();
            println!("Available audio devices:");
            for device in devices {
                println!("  {}", device);
            }
        }

        Commands::Test { message } => {
            println!("Running loopback test with message: {}", message);
            run_test(&message)?;
        }
    }

    Ok(())
}

fn send_data(data: &[u8], config: &Config) -> Result<()> {
    eprintln!("Preparing to send {} bytes...", data.len());

    let compressed = compress(data);
    eprintln!("Compressed to {} bytes", compressed.len());

    let ecc = ReedSolomonCodec::new()?;
    let encoded = ecc.encode(&compressed)?;
    eprintln!("ECC encoded to {} bytes", encoded.len());

    let packet = Packet::new(encoded)?;
    let packet_data = packet.serialize();
    eprintln!("Packet size: {} bytes", packet_data.len());

    let modulator = MFSKModulator::new(config.clone());
    let samples = modulator.modulate(&packet_data);
    let duration_ms = samples.len() as f32 / 48.0;
    eprintln!("Audio duration: {:.1} ms", duration_ms);

    let audio_output = AudioOutput::new()?;

    eprintln!("Transmitting...");
    audio_output.play_samples(samples)?;
    eprintln!("Transmission complete!");

    Ok(())
}

fn receive_data(config: &Config, timeout_secs: u32) -> Result<Vec<u8>> {
    eprintln!("Listening for transmission...");
    eprintln!("Mode: {:?}", config.mode);
    eprintln!("Timeout: {} seconds", timeout_secs);

    let audio_input = AudioInput::new()?;
    let mut demodulator = MFSKDemodulator::new(config.clone());

    let wake_detected = std::sync::Arc::new(std::sync::Mutex::new(false));
    let wake_detected_clone = wake_detected.clone();

    let samples = audio_input.record_until_complete(
        move |samples| {
            if samples.len() < 48000 {
                return false;
            }

            let mut temp_demod = MFSKDemodulator::new(config.clone());

            if temp_demod.detect_wake_up(samples).is_some() {
                *wake_detected_clone.lock().unwrap() = true;

                let end_check_start = samples.len().saturating_sub(24000);
                let end_samples = &samples[end_check_start..];

                let wake_mag = temp_demod.goertzel(end_samples, WAKE_UP_FREQUENCY);
                let noise: f32 = temp_demod
                    .get_frequencies()
                    .iter()
                    .map(|&f| temp_demod.goertzel(end_samples, f))
                    .sum::<f32>()
                    / 16.0;

                return wake_mag > noise * 2.0 && samples.len() > 96000;
            }

            false
        },
        timeout_secs * 1000,
    )?;

    eprintln!("Recorded {} samples, demodulating...", samples.len());

    let raw_data = demodulator
        .demodulate(&samples)
        .ok_or_else(|| anyhow::anyhow!("Failed to demodulate signal"))?;

    eprintln!("Demodulated {} bytes", raw_data.len());

    let packet = Packet::deserialize(&raw_data)?;
    eprintln!("Packet payload: {} bytes", packet.payload.len());

    let ecc = ReedSolomonCodec::new()?;
    let decoded = ecc.decode(&packet.payload)?;
    eprintln!("ECC decoded: {} bytes", decoded.len());

    let decompressed = decompress(&decoded)?;
    eprintln!("Decompressed: {} bytes", decompressed.len());

    Ok(decompressed)
}

fn run_test(message: &str) -> Result<()> {
    let config = Config::default();
    let data = message.as_bytes();

    let compressed = compress(data);
    let ecc = ReedSolomonCodec::new()?;
    let encoded = ecc.encode(&compressed)?;
    let encoded_len = encoded.len();
    let packet = Packet::new(encoded)?;
    let packet_data = packet.serialize();

    let modulator = MFSKModulator::new(config.clone());
    let samples = modulator.modulate(&packet_data);

    println!("Original: {} bytes", data.len());
    println!("Compressed: {} bytes", compressed.len());
    println!("ECC encoded: {} bytes", encoded_len);
    println!("Packet: {} bytes", packet_data.len());
    println!("Audio samples: {}", samples.len());
    println!("Duration: {:.1} ms", samples.len() as f32 / 48.0);

    let mut demodulator = MFSKDemodulator::new(config);
    let decoded_data = demodulator
        .demodulate(&samples)
        .ok_or_else(|| anyhow::anyhow!("Demodulation failed"))?;

    let decoded_packet = Packet::deserialize(&decoded_data)?;
    let ecc_decoded = ecc.decode(&decoded_packet.payload)?;
    let decompressed = decompress(&ecc_decoded)?;

    let result = String::from_utf8_lossy(&decompressed);
    println!("\nDecoded message: {}", result);

    if decompressed == data {
        println!("\n✓ Test PASSED: Messages match!");
    } else {
        println!("\n✗ Test FAILED: Messages don't match!");
        std::process::exit(1);
    }

    Ok(())
}

trait DemodulatorExt {
    fn get_frequencies(&self) -> Vec<f32>;
}

impl DemodulatorExt for MFSKDemodulator {
    fn get_frequencies(&self) -> Vec<f32> {
        let config = Config::default();
        let base_freq = config.mode.base_frequency();
        let step = config.mode.frequency_step();
        (0..16).map(|i| base_freq + (i as f32) * step).collect()
    }
}
