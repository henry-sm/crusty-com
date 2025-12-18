// Wait for Tauri API to be ready
console.log("main.js loaded");

// Function to log to status panel
function logToStatus(message) {
    const statusLog = document.getElementById('statusLog');
    if (statusLog) {
        const timestamp = new Date().toLocaleTimeString();
        statusLog.textContent += `[${timestamp}] ${message}\n`;
        statusLog.scrollTop = statusLog.scrollHeight;
    }
    console.log(message);
}

// Function to log opcodes to opcodes panel
function logOpcodes(opcode) {
    const opcodesDisplay = document.getElementById('opcodesDisplay');
    if (opcodesDisplay) {
        opcodesDisplay.textContent += `${opcode}\n`;
        // Keep only last 50 lines
        const lines = opcodesDisplay.textContent.split('\n');
        if (lines.length > 50) {
            opcodesDisplay.textContent = lines.slice(-50).join('\n');
        }
        opcodesDisplay.scrollTop = opcodesDisplay.scrollHeight;
    }
}

// Function to wait for Tauri to be ready
async function waitForTauri() {
    let attempts = 0;
    while (!window.__TAURI__ && attempts < 100) {
        await new Promise(resolve => setTimeout(resolve, 100));
        attempts++;
    }
    if (!window.__TAURI__) {
        console.error("Tauri API failed to load!");
        return false;
    }
    logToStatus("Tauri API loaded successfully");
    return true;
}

// Initialize when DOM is ready and Tauri is available
async function initApp() {
    const isTauriReady = await waitForTauri();
    if (!isTauriReady) {
        logToStatus("ERROR: Failed to initialize Tauri API");
        return;
    }

    const { invoke } = window.__TAURI__.core;
    const { open } = window.__TAURI__.dialog;
    const { listen } = window.__TAURI__.event;

    const canvas = document.getElementById('screen');
    const ctx = canvas.getContext('2d');
    const imageData = ctx.createImageData(256, 224);
    const loadRomBtn = document.getElementById('loadRomBtn');

    logToStatus("Canvas initialized");
    logToStatus("Load ROM button ready");

    // Initialize Web Audio API for sound playback
    const audioContext = new (window.AudioContext || window.webkitAudioContext)();
    const audioBuffer = [];
    const audioSampleRate = 32000;  // Match APU sample rate
    let audioSource = null;
    
    logToStatus(`Audio context initialized: sample rate ${audioContext.sampleRate}Hz`);
    
    // Function to queue audio samples for playback
    async function playAudio() {
        while (true) {
            try {
                const samples = await invoke('get_audio');
                if (samples && samples.length > 0) {
                    // Queue audio data
                    audioBuffer.push(...samples);
                    
                    // If we have enough samples, create and play an audio buffer
                    if (audioBuffer.length > audioSampleRate * 0.05) {  // 50ms of audio
                        const audioData = audioBuffer.splice(0, audioSampleRate * 0.05);
                        
                        // Create audio buffer
                        const length = audioData.length / 2;  // Stereo
                        const abuf = audioContext.createBuffer(2, length, audioSampleRate);
                        const leftChannel = abuf.getChannelData(0);
                        const rightChannel = abuf.getChannelData(1);
                        
                        // Fill channels (convert i16 to float)
                        for (let i = 0; i < length; i++) {
                            leftChannel[i] = audioData[i * 2] / 32768.0;
                            rightChannel[i] = audioData[i * 2 + 1] / 32768.0;
                        }
                        
                        // Play the buffer
                        audioSource = audioContext.createBufferSource();
                        audioSource.buffer = abuf;
                        audioSource.connect(audioContext.destination);
                        audioSource.start(0);
                    }
                }
            } catch (e) {
                // Silently ignore audio errors
            }
            
            // Check audio every 16ms
            await new Promise(resolve => setTimeout(resolve, 16));
        }
    }
    
    // Start audio playback thread (non-blocking)
    playAudio();

    // Listen for log messages from the backend
    listen('log-message', (event) => {
        logToStatus(event.payload);
    });

    // Keymap: keyboard key -> SNES button bit
    // Bit positions: 0=Right, 1=Left, 2=Down, 3=Up, 4=START, 5=SELECT, 6=Y, 7=B
    const keyMap = {
        'ArrowRight': 0,
        'ArrowLeft': 1,
        'ArrowDown': 2,
        'ArrowUp': 3,
        'Enter': 4,
        'Backspace': 5,
        'KeyA': 6,  // Y button
        'KeyZ': 7,  // B button
    };

    // Listen for frame updates from the Rust backend
    // Instead of events, we poll for frames
    // This is more reliable than event serialization for large data
    
    async function renderLoop() {
        while (true) {
            try {
                const frameData = await invoke('get_frame');
                if (frameData && frameData.length === 256 * 224) {
                    // Convert array to Uint32Array (already in correct format)
                    const buffer = new Uint8ClampedArray(new Uint32Array(frameData).buffer);
                    imageData.data.set(buffer);
                    ctx.putImageData(imageData, 0, 0);
                }
            } catch (e) {
                logToStatus(`Frame rendering error: ${e}`);
            }
            
            // ~60 FPS (frame rate matching emulation)
            await new Promise(resolve => setTimeout(resolve, 16));
        }
    }
    
    // Start the render loop
    renderLoop();

    // Handle keyboard input
    document.addEventListener('keydown', async (e) => {
        const button = keyMap[e.code];
        if (button !== undefined) {
            await invoke('press_button', { button });
            e.preventDefault();
        }
    });

    document.addEventListener('keyup', async (e) => {
        const button = keyMap[e.code];
        if (button !== undefined) {
            await invoke('release_button', { button });
            e.preventDefault();
        }
    });

    // Handle the "Load ROM" button click
    if (loadRomBtn) {
        logToStatus("Waiting for ROM load...");
        loadRomBtn.addEventListener('click', async () => {
            logToStatus("Load ROM button clicked!");
            try {
                logToStatus("Opening file dialog...");
                const romPath = await open({
                    title: 'Select a SNES ROM',
                    multiple: false,
                    filters: [{ name: 'SNES ROM', extensions: ['sfc', 'smc', 'nes'] }]
                });

                logToStatus(`Dialog result: ${romPath}`);
                if (romPath) {
                    // Call the 'load_rom' command in the Rust backend
                    try {
                        logToStatus(`Loading ROM from: ${romPath}`);
                        await invoke('load_rom', { path: romPath });
                        logToStatus(`ROM "${romPath}" loaded successfully!`);
                    } catch (e) {
                        logToStatus(`ERROR: Failed to load ROM: ${e}`);
                    }
                } else {
                    logToStatus("ROM load cancelled by user");
                }
            } catch (e) {
                logToStatus(`ERROR: File dialog error: ${e}`);
            }
        });
    } else {
        logToStatus("ERROR: Load ROM button not found in DOM!");
    }
}

// Start the app when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initApp);
} else {
    initApp();
}