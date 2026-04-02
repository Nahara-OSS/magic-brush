/**
 * Special module that handles WebAssembly loading (because `deno bundle` does not support WebAssembly import yet).
 * 
 * @module
 */

import wasmBinary from "./pkg/magic_brush_demo_bg.wasm" with { type: "bytes" };
import * as ModuleBG from "./pkg/magic_brush_demo_bg.js";

const { instance } = await WebAssembly.instantiate(wasmBinary, {
    "./magic_brush_demo_bg.js": ModuleBG
});

ModuleBG.__wbg_set_wasm(instance.exports);

export * from "./pkg/magic_brush_demo_bg.js";