// Wait for Tauri API to be ready
console.log("main.js loaded");

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
    console.log("Tauri API loaded successfully");
    return true;
}

// Initialize when DOM is ready and Tauri is available
async function initApp() {
    const isTauriReady = await waitForTauri();
    if (!isTauriReady) {
        alert("Failed to initialize Tauri API");
        return;
    }

    const { invoke } = window.__TAURI__.core;
    const { listen } = window.__TAURI__.event;
    const { open } = window.__TAURI__.dialog;

    const canvas = document.getElementById('screen');
    const ctx = canvas.getContext('2d');
    const imageData = ctx.createImageData(256, 224);
    const loadRomBtn = document.getElementById('loadRomBtn');

    console.log("Canvas:", canvas);
    console.log("Load ROM Button:", loadRomBtn);

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
    await listen('frame-update', (event) => {
        const frameData = new Uint32Array(event.payload);
        const buffer = new Uint8ClampedArray(frameData.buffer);
        imageData.data.set(buffer);
        ctx.putImageData(imageData, 0, 0);
    });

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
        console.log("Attaching click listener to load ROM button");
        loadRomBtn.addEventListener('click', async () => {
            console.log("Load ROM button clicked!");
            try {
                console.log("Opening file dialog...");
                const romPath = await open({
                    title: 'Select a SNES ROM',
                    multiple: false,
                    filters: [{ name: 'SNES ROM', extensions: ['sfc', 'smc', 'nes'] }]
                });

                console.log("Dialog result:", romPath);
                if (romPath) {
                    // Call the 'load_rom' command in the Rust backend
                    try {
                        console.log(`Loading ROM from: ${romPath}`);
                        await invoke('load_rom', { path: romPath });
                        console.log(`ROM "${romPath}" loaded successfully.`);
                        alert(`ROM loaded successfully!`);
                    } catch (e) {
                        console.error(`Failed to invoke load_rom: ${e}`);
                        alert(`Failed to load ROM: ${e}`);
                    }
                } else {
                    console.log("No ROM selected (dialog cancelled)");
                }
            } catch (e) {
                console.error(`File dialog error: ${e}`);
                alert(`File dialog error: ${e}`);
            }
        });
    } else {
        console.error("Load ROM button not found in DOM!");
    }
}

// Start the app when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initApp);
} else {
    initApp();
}