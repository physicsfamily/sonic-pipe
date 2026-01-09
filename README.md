# Sonic-Pipe

> **Transfer data between devices using sound waves — no WiFi, no Bluetooth, no cables.**

[![Live Demo](https://img.shields.io/badge/demo-sonic.graviton.dev-00ff88?style=for-the-badge)](https://sonic.graviton.dev)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)

Sonic-Pipe is an **acoustic modem** that transmits data through sound waves using your device's speaker and microphone. Perfect for:

- **Air-gapped security** — Transfer secrets without network exposure
- **Offline sharing** — Share text between devices with no internet
- **Cross-platform** — Works on any device with a speaker/microphone
- **Privacy-first** — 100% client-side, no data sent to servers

## How It Works

```
Your Device                          Other Device
┌─────────────┐    🔊 Sound Waves    ┌─────────────┐
│  Speaker    │ ──────────────────▶  │ Microphone  │
│  (Transmit) │                      │  (Receive)  │
└─────────────┘                      └─────────────┘
```

Data is encoded into audio tones using **16-tone MFSK modulation**, transmitted through air, and decoded on the receiving device. Built-in error correction ensures reliability even in noisy environments.

## Features

- **Two Transmission Modes**
  - **Audible** (1-3 kHz): Higher throughput, audible chirps
  - **Ultrasonic** (17-20 kHz): Near-silent, stealthy transfer
- **Error Resilient** — Reed-Solomon error correction recovers from noise
- **Compressed** — LZ4 compression for efficient transfer
- **CLI Tool** — Unix pipe-friendly for scripting
- **Web App** — Browser-based, no installation needed
- **Open Source** — MIT licensed, fully auditable

## Installation

### CLI (Rust)

```bash
# Build from source
cargo build --release

# Install globally
cargo install --path .
```

### Web

Simply open `web/index.html` in a modern browser, or serve the `web/` directory:

```bash
cd web && python3 -m http.server 8080
```

## Usage

### CLI

```bash
# Send a message
echo "Hello, World!" | sonic-pipe send

# Send in ultrasonic mode
echo "Secret message" | sonic-pipe send --ultrasonic

# Receive data
sonic-pipe receive > received.txt

# Receive in ultrasonic mode
sonic-pipe receive --ultrasonic > received.txt

# Test the transmission (loopback)
sonic-pipe test "Hello, Sonic-Pipe!"

# List audio devices
sonic-pipe devices
```

### Web Interface

1. Open the web interface in your browser
2. Select transmission mode (Audible or Ultrasonic)
3. **To Send**: Enter your message and click "Transmit"
4. **To Receive**: Click "Start Listening" and watch the spectrogram

### JavaScript API

```html
<script src="sonic-pipe.js"></script>
<script>
  const driver = new SonicPipe({ ultrasonic: false });
  
  // Send data
  await driver.send("Hello World");
  
  // Listen for data
  driver.on('message', (msg) => {
    console.log("Received:", msg);
  });
  await driver.startListening();
</script>
```

## Protocol Specification

### Audio Physics

| Parameter | Audible Mode | Ultrasonic Mode |
|-----------|--------------|-----------------|
| Base Frequency | 1 kHz | 17 kHz |
| Frequency Step | 100 Hz | 150 Hz |
| Frequency Range | 1-2.5 kHz | 17-19.4 kHz |
| Sample Rate | 48 kHz | 48 kHz |
| Symbol Duration | 50 ms (default) | 50 ms (default) |

### Packet Structure

```
[ WAKE_UP_TONE ] + [ HEADER ] + [ PAYLOAD ] + [ CRC32 ]
```

- **Wake-up Tone**: 18.5 kHz, 100ms - signals start of transmission
- **Header**: 4 bytes (version, payload length, flags)
- **Payload**: Compressed and ECC-encoded data
- **CRC32**: 4-byte checksum for integrity verification

### Data Pipeline

```
Input → LZ4 Compress → Reed-Solomon ECC → Packet → MFSK Modulate → Audio
```

## Security Considerations

⚠️ **Sonic-Pipe is a physical layer (Layer 1) transport.** It does NOT encrypt data.

For sensitive data, encrypt before transmission:

```bash
# Encrypt with GPG before sending
gpg -e -r recipient@email.com message.txt | sonic-pipe send

# Decrypt after receiving
sonic-pipe receive | gpg -d > decrypted.txt
```

## Limitations

- **Range**: Works best within 1-3 meters
- **Interference**: Background noise may cause bit errors
- **Throughput**: ~10-50 bytes/second depending on mode and conditions
- **Line-of-sight**: Direct path between speaker and microphone recommended

## Development

### Building CLI

```bash
cargo build --release
```

### Running Tests

```bash
cargo test
```

### Building WASM (optional)

```bash
wasm-pack build --target web
```

## Use Cases

- **Security professionals** — Transfer credentials to air-gapped systems
- **Developers** — Quick text sharing during demos without network setup
- **Privacy advocates** — Share sensitive info without digital trail
- **Hackers & makers** — Experimental data exfiltration research

## Browser Support

| Browser | Status |
|---------|--------|
| Chrome 66+ | ✅ Full support |
| Firefox 76+ | ✅ Full support |
| Safari 14.1+ | ✅ Full support |
| Edge 79+ | ✅ Full support |

Requires Web Audio API and getUserMedia for microphone access.

## License

MIT License - See [LICENSE](LICENSE) for details.

## Contributing

Contributions welcome! Please open an issue or submit a pull request.

---

<p align="center">
  <b>🔊 Try it now at <a href="https://sonic.graviton.dev">sonic.graviton.dev</a></b>
</p>
