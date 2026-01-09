document.addEventListener('DOMContentLoaded', () => {
    const sonicPipe = new SonicPipe();
    
    const tabs = document.querySelectorAll('.tab');
    const sendTab = document.getElementById('send-tab');
    const receiveTab = document.getElementById('receive-tab');
    const messageInput = document.getElementById('message-input');
    const sendBtn = document.getElementById('send-btn');
    const receiveBtn = document.getElementById('receive-btn');
    const stopBtn = document.getElementById('stop-btn');
    const sendStatus = document.getElementById('send-status');
    const receiveStatus = document.getElementById('receive-status');
    const receivedMessage = document.getElementById('received-message');
    const spectrogram = document.getElementById('spectrogram');
    const symbolDurationSlider = document.getElementById('symbol-duration');
    const symbolDurationValue = document.getElementById('symbol-duration-value');
    const volumeSlider = document.getElementById('volume');
    const volumeValue = document.getElementById('volume-value');
    const modeRadios = document.querySelectorAll('input[name="mode"]');

    const ctx = spectrogram.getContext('2d');
    let spectrogramData = [];
    const spectrogramHeight = 200;
    const spectrogramWidth = spectrogram.width;

    tabs.forEach(tab => {
        tab.addEventListener('click', () => {
            tabs.forEach(t => t.classList.remove('active'));
            tab.classList.add('active');

            if (tab.dataset.tab === 'send') {
                sendTab.classList.remove('hidden');
                receiveTab.classList.add('hidden');
            } else {
                sendTab.classList.add('hidden');
                receiveTab.classList.remove('hidden');
            }
        });
    });

    modeRadios.forEach(radio => {
        radio.addEventListener('change', (e) => {
            sonicPipe.setMode(e.target.value === 'ultrasonic');
        });
    });

    symbolDurationSlider.addEventListener('input', (e) => {
        const value = parseInt(e.target.value);
        symbolDurationValue.textContent = value;
        sonicPipe.setSymbolDuration(value);
    });

    volumeSlider.addEventListener('input', (e) => {
        const value = parseInt(e.target.value);
        volumeValue.textContent = value + '%';
        sonicPipe.setVolume(value / 100);
    });

    function setStatus(element, message, type = 'info') {
        element.textContent = message;
        element.className = 'status ' + type;
    }

    sendBtn.addEventListener('click', async () => {
        const message = messageInput.value.trim();

        if (!message) {
            setStatus(sendStatus, 'Please enter a message to send', 'error');
            return;
        }

        sendBtn.disabled = true;
        sendBtn.classList.add('transmitting');
        setStatus(sendStatus, 'Transmitting...', 'info');

        try {
            await sonicPipe.send(message);
            setStatus(sendStatus, 'Transmission complete!', 'success');
        } catch (err) {
            setStatus(sendStatus, 'Error: ' + err.message, 'error');
        } finally {
            sendBtn.disabled = false;
            sendBtn.classList.remove('transmitting');
        }
    });

    receiveBtn.addEventListener('click', async () => {
        receiveBtn.classList.add('hidden');
        stopBtn.classList.remove('hidden');
        receivedMessage.textContent = '';
        spectrogramData = [];
        clearSpectrogram();
        setStatus(receiveStatus, 'Listening for transmission...', 'info');

        try {
            sonicPipe.on('message', (msg) => {
                receivedMessage.textContent = msg;
                setStatus(receiveStatus, 'Message received!', 'success');
            });

            sonicPipe.on('spectrum', (data) => {
                updateSpectrogram(data);
            });

            await sonicPipe.startListening();
        } catch (err) {
            setStatus(receiveStatus, 'Error: ' + err.message, 'error');
            receiveBtn.classList.remove('hidden');
            stopBtn.classList.add('hidden');
        }
    });

    stopBtn.addEventListener('click', () => {
        sonicPipe.stopListening();
        receiveBtn.classList.remove('hidden');
        stopBtn.classList.add('hidden');
        setStatus(receiveStatus, 'Listening stopped', 'info');
    });

    function clearSpectrogram() {
        ctx.fillStyle = '#000';
        ctx.fillRect(0, 0, spectrogramWidth, spectrogramHeight);
    }

    function updateSpectrogram(frequencyData) {
        const column = new Uint8Array(frequencyData.length);
        column.set(frequencyData);
        spectrogramData.push(column);

        if (spectrogramData.length > spectrogramWidth) {
            spectrogramData.shift();
        }

        drawSpectrogram();
    }

    function drawSpectrogram() {
        const imageData = ctx.createImageData(spectrogramWidth, spectrogramHeight);
        const data = imageData.data;

        for (let x = 0; x < spectrogramData.length; x++) {
            const column = spectrogramData[x];
            const displayBins = Math.min(column.length, spectrogramHeight);

            for (let y = 0; y < displayBins; y++) {
                const intensity = column[y];
                const pixelY = spectrogramHeight - 1 - y;
                const idx = (pixelY * spectrogramWidth + x) * 4;

                const [r, g, b] = intensityToColor(intensity);
                data[idx] = r;
                data[idx + 1] = g;
                data[idx + 2] = b;
                data[idx + 3] = 255;
            }
        }

        ctx.putImageData(imageData, 0, 0);

        drawFrequencyMarkers();
    }

    function intensityToColor(intensity) {
        const normalized = intensity / 255;

        if (normalized < 0.2) {
            const t = normalized / 0.2;
            return [0, Math.floor(20 * t), Math.floor(30 * t)];
        } else if (normalized < 0.5) {
            const t = (normalized - 0.2) / 0.3;
            return [0, Math.floor(20 + 215 * t), Math.floor(30 + 50 * t)];
        } else if (normalized < 0.8) {
            const t = (normalized - 0.5) / 0.3;
            return [Math.floor(255 * t), 235, Math.floor(80 - 80 * t)];
        } else {
            const t = (normalized - 0.8) / 0.2;
            return [255, Math.floor(235 + 20 * t), Math.floor(200 * t)];
        }
    }

    function drawFrequencyMarkers() {
        ctx.strokeStyle = 'rgba(0, 255, 136, 0.3)';
        ctx.lineWidth = 1;
        ctx.setLineDash([2, 4]);

        const frequencies = sonicPipe.getFrequencies();
        const nyquist = 48000 / 2;
        const binCount = 2048;

        frequencies.forEach(freq => {
            const bin = Math.floor((freq / nyquist) * binCount);
            const y = spectrogramHeight - 1 - Math.floor((bin / binCount) * spectrogramHeight);

            if (y >= 0 && y < spectrogramHeight) {
                ctx.beginPath();
                ctx.moveTo(0, y);
                ctx.lineTo(spectrogramWidth, y);
                ctx.stroke();
            }
        });

        ctx.setLineDash([]);
    }

    clearSpectrogram();
});
