/**
 * Sonic-Pipe JavaScript Library
 * Pure JS implementation for acoustic data transfer
 */

class SonicPipe {
    constructor(options = {}) {
        this.sampleRate = 48000;
        this.symbolDuration = options.symbolDuration || 50;
        this.volume = options.volume || 0.5;
        this.ultrasonic = options.ultrasonic || false;
        
        this.audioContext = null;
        this.analyser = null;
        this.mediaStream = null;
        this.isListening = false;
        this.onMessage = null;
        this.onSpectrumData = null;
        
        this.frequencies = this._generateFrequencies();
        this.wakeUpFrequency = 18500;
        this.wakeUpDuration = 100;
    }

    _generateFrequencies() {
        const baseFreq = this.ultrasonic ? 17000 : 1000;
        const step = this.ultrasonic ? 150 : 100;
        return Array.from({ length: 16 }, (_, i) => baseFreq + i * step);
    }

    setMode(ultrasonic) {
        this.ultrasonic = ultrasonic;
        this.frequencies = this._generateFrequencies();
    }

    setSymbolDuration(ms) {
        this.symbolDuration = ms;
    }

    setVolume(vol) {
        this.volume = Math.max(0, Math.min(1, vol));
    }

    async init() {
        if (!this.audioContext) {
            this.audioContext = new (window.AudioContext || window.webkitAudioContext)({
                sampleRate: this.sampleRate
            });
        }
        
        if (this.audioContext.state === 'suspended') {
            await this.audioContext.resume();
        }
        
        return this;
    }

    _compress(data) {
        const result = [];
        let i = 0;
        
        while (i < data.length) {
            let runLength = 1;
            while (i + runLength < data.length && 
                   data[i] === data[i + runLength] && 
                   runLength < 255) {
                runLength++;
            }
            
            if (runLength > 3) {
                result.push(0xFF, data[i], runLength);
                i += runLength;
            } else {
                if (data[i] === 0xFF) {
                    result.push(0xFF, 0xFF, 1);
                } else {
                    result.push(data[i]);
                }
                i++;
            }
        }
        
        return new Uint8Array(result);
    }

    _decompress(data) {
        const result = [];
        let i = 0;
        
        while (i < data.length) {
            if (data[i] === 0xFF && i + 2 < data.length) {
                const value = data[i + 1];
                const count = data[i + 2];
                
                if (value === 0xFF && count === 1) {
                    result.push(0xFF);
                } else {
                    for (let j = 0; j < count; j++) {
                        result.push(value);
                    }
                }
                i += 3;
            } else {
                result.push(data[i]);
                i++;
            }
        }
        
        return new Uint8Array(result);
    }

    _crc32(data) {
        let crc = 0xFFFFFFFF;
        const table = this._getCrc32Table();
        
        for (let i = 0; i < data.length; i++) {
            crc = (crc >>> 8) ^ table[(crc ^ data[i]) & 0xFF];
        }
        
        return (crc ^ 0xFFFFFFFF) >>> 0;
    }

    _getCrc32Table() {
        if (!SonicPipe._crc32Table) {
            const table = new Uint32Array(256);
            for (let i = 0; i < 256; i++) {
                let c = i;
                for (let j = 0; j < 8; j++) {
                    c = (c & 1) ? (0xEDB88320 ^ (c >>> 1)) : (c >>> 1);
                }
                table[i] = c;
            }
            SonicPipe._crc32Table = table;
        }
        return SonicPipe._crc32Table;
    }

    _createPacket(data) {
        const compressed = this._compress(data);
        const checksum = this._crc32(compressed);
        
        const packet = new Uint8Array(4 + compressed.length + 4);
        packet[0] = 1;
        packet[1] = (compressed.length >> 8) & 0xFF;
        packet[2] = compressed.length & 0xFF;
        packet[3] = 0;
        packet.set(compressed, 4);
        
        packet[4 + compressed.length] = (checksum >> 24) & 0xFF;
        packet[4 + compressed.length + 1] = (checksum >> 16) & 0xFF;
        packet[4 + compressed.length + 2] = (checksum >> 8) & 0xFF;
        packet[4 + compressed.length + 3] = checksum & 0xFF;
        
        return packet;
    }

    _parsePacket(data) {
        if (data.length < 8) {
            throw new Error('Packet too short');
        }
        
        const payloadLen = (data[1] << 8) | data[2];
        
        if (data.length < 4 + payloadLen + 4) {
            throw new Error('Incomplete packet');
        }
        
        const payload = data.slice(4, 4 + payloadLen);
        const storedChecksum = (data[4 + payloadLen] << 24) |
                               (data[4 + payloadLen + 1] << 16) |
                               (data[4 + payloadLen + 2] << 8) |
                               data[4 + payloadLen + 3];
        
        const computedChecksum = this._crc32(payload);
        
        if (storedChecksum !== computedChecksum) {
            throw new Error('Checksum mismatch');
        }
        
        return this._decompress(payload);
    }

    _generateTone(frequency, durationMs) {
        const numSamples = Math.floor(this.sampleRate * durationMs / 1000);
        const samples = new Float32Array(numSamples);
        const fadeSamples = Math.floor(this.sampleRate * 0.005);
        
        for (let i = 0; i < numSamples; i++) {
            const t = i / this.sampleRate;
            let sample = Math.sin(2 * Math.PI * frequency * t) * this.volume;
            
            if (i < fadeSamples) {
                sample *= i / fadeSamples;
            } else if (i > numSamples - fadeSamples) {
                sample *= (numSamples - i) / fadeSamples;
            }
            
            samples[i] = sample;
        }
        
        return samples;
    }

    _modulate(data) {
        const allSamples = [];
        
        allSamples.push(...this._generateTone(this.wakeUpFrequency, this.wakeUpDuration));
        
        const silenceSamples = Math.floor(this.sampleRate * 0.02);
        allSamples.push(...new Float32Array(silenceSamples));
        
        for (const byte of data) {
            const highNibble = (byte >> 4) & 0x0F;
            const lowNibble = byte & 0x0F;
            
            allSamples.push(...this._generateTone(this.frequencies[highNibble], this.symbolDuration));
            allSamples.push(...this._generateTone(this.frequencies[lowNibble], this.symbolDuration));
        }
        
        allSamples.push(...this._generateTone(this.wakeUpFrequency, this.wakeUpDuration));
        
        return new Float32Array(allSamples);
    }

    async send(message) {
        await this.init();
        
        let data;
        if (typeof message === 'string') {
            data = new TextEncoder().encode(message);
        } else if (message instanceof Uint8Array) {
            data = message;
        } else {
            data = new Uint8Array(message);
        }
        
        const packet = this._createPacket(data);
        const samples = this._modulate(packet);
        
        const buffer = this.audioContext.createBuffer(1, samples.length, this.sampleRate);
        buffer.getChannelData(0).set(samples);
        
        const source = this.audioContext.createBufferSource();
        source.buffer = buffer;
        source.connect(this.audioContext.destination);
        
        return new Promise((resolve) => {
            source.onended = () => {
                resolve();
            };
            source.start();
        });
    }

    _goertzel(samples, targetFreq) {
        const n = samples.length;
        const k = Math.round(targetFreq * n / this.sampleRate);
        const omega = (2 * Math.PI * k) / n;
        const coeff = 2 * Math.cos(omega);
        
        let s0 = 0, s1 = 0, s2 = 0;
        
        for (let i = 0; i < n; i++) {
            s0 = samples[i] + coeff * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        
        return Math.sqrt(s1 * s1 + s2 * s2 - s1 * s2 * coeff);
    }

    _detectSymbol(samples) {
        let maxMag = 0;
        let detectedIndex = 0;
        
        for (let i = 0; i < this.frequencies.length; i++) {
            const mag = this._goertzel(samples, this.frequencies[i]);
            if (mag > maxMag) {
                maxMag = mag;
                detectedIndex = i;
            }
        }
        
        return detectedIndex;
    }

    _detectWakeUp(samples) {
        const windowSize = Math.floor(this.sampleRate * this.wakeUpDuration / 1000 / 2);
        const step = Math.floor(windowSize / 4);
        const threshold = 0.3;
        
        for (let i = 0; i < samples.length - windowSize; i += step) {
            const window = samples.slice(i, i + windowSize);
            const wakeMag = this._goertzel(window, this.wakeUpFrequency);
            
            let noiseMag = 0;
            for (const freq of this.frequencies) {
                noiseMag += this._goertzel(window, freq);
            }
            noiseMag /= this.frequencies.length;
            
            if (wakeMag > threshold && wakeMag > noiseMag * 3) {
                return i + windowSize;
            }
        }
        
        return -1;
    }

    _demodulate(samples) {
        const startPos = this._detectWakeUp(samples);
        if (startPos < 0) {
            return null;
        }
        
        const symbolSamples = Math.floor(this.sampleRate * this.symbolDuration / 1000);
        let pos = startPos + Math.floor(this.sampleRate * 0.02);
        
        const nibbles = [];
        
        while (pos + symbolSamples <= samples.length) {
            const window = samples.slice(pos, pos + symbolSamples);
            
            const wakeMag = this._goertzel(window, this.wakeUpFrequency);
            let dataMag = 0;
            for (const freq of this.frequencies) {
                dataMag = Math.max(dataMag, this._goertzel(window, freq));
            }
            
            if (wakeMag > dataMag * 2) {
                break;
            }
            
            nibbles.push(this._detectSymbol(window));
            pos += symbolSamples;
        }
        
        const bytes = [];
        for (let i = 0; i < nibbles.length - 1; i += 2) {
            bytes.push((nibbles[i] << 4) | (nibbles[i + 1] & 0x0F));
        }
        
        return new Uint8Array(bytes);
    }

    async startListening() {
        await this.init();
        
        try {
            this.mediaStream = await navigator.mediaDevices.getUserMedia({ audio: true });
        } catch (err) {
            throw new Error('Microphone access denied');
        }
        
        const source = this.audioContext.createMediaStreamSource(this.mediaStream);
        
        this.analyser = this.audioContext.createAnalyser();
        this.analyser.fftSize = 4096;
        this.analyser.smoothingTimeConstant = 0.5;
        source.connect(this.analyser);
        
        this.isListening = true;
        
        const bufferLength = this.analyser.frequencyBinCount;
        const dataArray = new Uint8Array(bufferLength);
        
        const recordedSamples = [];
        const scriptProcessor = this.audioContext.createScriptProcessor(4096, 1, 1);
        let wakeUpDetected = false;
        let silenceCount = 0;
        
        scriptProcessor.onaudioprocess = (e) => {
            if (!this.isListening) return;
            
            const inputData = e.inputBuffer.getChannelData(0);
            recordedSamples.push(...inputData);
            
            if (this.onSpectrumData) {
                this.analyser.getByteFrequencyData(dataArray);
                this.onSpectrumData(dataArray);
            }
            
            if (recordedSamples.length > this.sampleRate) {
                const recentSamples = new Float32Array(recordedSamples.slice(-this.sampleRate));
                
                if (!wakeUpDetected) {
                    if (this._detectWakeUp(recentSamples) >= 0) {
                        wakeUpDetected = true;
                        silenceCount = 0;
                    }
                } else {
                    const rms = Math.sqrt(
                        inputData.reduce((sum, x) => sum + x * x, 0) / inputData.length
                    );
                    
                    if (rms < 0.01) {
                        silenceCount++;
                    } else {
                        silenceCount = 0;
                    }
                    
                    if (silenceCount > 10 || recordedSamples.length > this.sampleRate * 30) {
                        this._processRecording(new Float32Array(recordedSamples));
                        recordedSamples.length = 0;
                        wakeUpDetected = false;
                    }
                }
                
                if (recordedSamples.length > this.sampleRate * 60) {
                    recordedSamples.splice(0, recordedSamples.length - this.sampleRate * 30);
                }
            }
        };
        
        source.connect(scriptProcessor);
        scriptProcessor.connect(this.audioContext.destination);
        
        this._scriptProcessor = scriptProcessor;
        this._source = source;
    }

    _processRecording(samples) {
        try {
            const rawData = this._demodulate(samples);
            if (rawData && rawData.length > 0) {
                const payload = this._parsePacket(rawData);
                const message = new TextDecoder().decode(payload);
                
                if (this.onMessage) {
                    this.onMessage(message);
                }
            }
        } catch (err) {
            console.log('Demodulation attempt failed:', err.message);
        }
    }

    stopListening() {
        this.isListening = false;
        
        if (this._scriptProcessor) {
            this._scriptProcessor.disconnect();
            this._scriptProcessor = null;
        }
        
        if (this._source) {
            this._source.disconnect();
            this._source = null;
        }
        
        if (this.mediaStream) {
            this.mediaStream.getTracks().forEach(track => track.stop());
            this.mediaStream = null;
        }
    }

    on(event, callback) {
        if (event === 'message') {
            this.onMessage = callback;
        } else if (event === 'spectrum') {
            this.onSpectrumData = callback;
        }
        return this;
    }

    getFrequencies() {
        return this.frequencies;
    }

    getWakeUpFrequency() {
        return this.wakeUpFrequency;
    }
}

if (typeof module !== 'undefined' && module.exports) {
    module.exports = SonicPipe;
}

if (typeof window !== 'undefined') {
    window.SonicPipe = SonicPipe;
}
