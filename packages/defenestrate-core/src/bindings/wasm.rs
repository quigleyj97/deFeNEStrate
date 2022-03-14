/// WASM front-end for the NES emulator
use crate::devices::cpu::WithCpu;
use crate::devices::nes::Nes;
use console_error_panic_hook;
use js_sys::Uint8Array;
use std::panic;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub struct NesEmulator {
    nes: Nes,
}

#[wasm_bindgen]
impl NesEmulator {
    #[wasm_bindgen(constructor)]
    pub fn new(buf: &[u8]) -> NesEmulator {
        let mut nes = Nes::new_from_buf(buf);
        nes.cpu_mut().state.pc = 0xC000;
        return NesEmulator { nes };
    }

    #[wasm_bindgen]
    pub fn dbg_step_cpu(&mut self) {
        for i in 0..5000 {
            alert(&format!("L{:04} {}", i, &self.nes.dbg_step_cpu()));
        }
    }

    #[wasm_bindgen]
    pub fn step_frame(&mut self) -> Uint8Array {
        let buf = self.nes.tick_frame();
        return Uint8Array::from(buf);
    }
}

#[wasm_bindgen]
pub fn hello(buf: &[u8]) {
    alert("Hello, world!");
    let mut nes = Nes::new_from_buf(buf);
    nes.cpu_mut().state.pc = 0xC000;

    let mut line = 1;

    for i in 0..5000 {
        alert(&format!("L{:04} {}", line, &nes.dbg_step_cpu()));
        line += 1;
    }
}

/// Installs a global panic handler to make debugging easier
#[wasm_bindgen]
pub fn init_debug_hooks() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}
