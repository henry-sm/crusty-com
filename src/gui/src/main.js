import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

const canvas = document.getElementById('screen');
const ctx = canvas.getContext('2d');
const imageData = ctx.createImageData(256, 224);
const loadRomBtn = document.getElementById('loadRomBtn');

// Listen for frame updates from the Rust backend
listen('frame-update', (event) => {
    const frameData = new Uint32Array(event.payload);
    const buffer = new Uint8ClampedArray(frameData.buffer);
    imageData.data.set(buffer);
    ctx.putImageData(imageData, 0, 0);
});

// Handle the "Load ROM" button click
loadRomBtn.addEventListener('click', async () => {
    const rom = await open({
        title: 'Select a SNES ROM',
        multiple: false,
        filters: [{ name: 'SNES ROM', extensions: ['sfc', 'smc'] }]
    });

    if (rom) {
        // Call the 'load_rom' command in the Rust backend
        await invoke('load_rom', { path: rom.path });
        console.log(`ROM "${rom.path}" loaded.`);
    }
});