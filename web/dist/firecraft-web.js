/* @ts-self-types="./firecraft-web.d.ts" */
import * as bg from "./firecraft-web_bg.js";

async function init() {
    // Fetch the WASM binary
    const resp = await fetch(new URL('./firecraft-web_bg.wasm', import.meta.url));
    if (!resp.ok) throw new Error('Failed to fetch WASM: ' + resp.status);
    
    // Read response as ArrayBuffer (required for WebAssembly.instantiate)
    const wasmArrayBuffer = await resp.arrayBuffer();
    
    // Build the imports object from bg module exports
    const imports = {};
    for (const key of Object.keys(bg)) {
        if (typeof bg[key] === 'function') {
            imports[key] = bg[key];
        }
    }
    
    // Instantiate WASM with imports
    const wasmModule = await WebAssembly.instantiate(wasmArrayBuffer, {
        './firecraft-web_bg.js': imports
    });
    
    const wasmExports = wasmModule.instance.exports;
    
    // Set up the wasm variable in the bg module
    bg.__wbg_set_wasm(wasmExports);
    
    // Initialize WASM (replaces stubs with real exports)
    wasmExports.__wbindgen_start();
    
    // Return start function
    return wasmExports.start;
}

export default init;
export { init as start };
