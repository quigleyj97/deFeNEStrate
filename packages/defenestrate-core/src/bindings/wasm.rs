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

#[wasm_bindgen(getter_with_clone)]
pub struct EmulatorDebugState {
    pub nametable: Uint8Array,
    pub palette: Uint8Array,
    pub chr: Uint8Array,
}

#[wasm_bindgen]
impl NesEmulator {
    #[wasm_bindgen(constructor)]
    pub fn new(buf: &[u8]) -> NesEmulator {
        let mut nes = Nes::new_from_buf(buf);
        return NesEmulator { nes };
    }

    #[wasm_bindgen]
    pub fn dbg_step_cpu(&mut self) -> String {
        return format!("{}", &self.nes.dbg_step_cpu());
    }

    #[wasm_bindgen]
    pub fn reset(&mut self) {
        self.nes.reset();
    }

    #[wasm_bindgen]
    pub fn dump_debug_data(&self) -> EmulatorDebugState {
        let (nametable, palette, chr) = self.nes.dump_debug_data();
        return EmulatorDebugState {
            nametable: Uint8Array::from(nametable),
            palette: Uint8Array::from(palette),
            chr: Uint8Array::from(chr),
        };
    }

    #[wasm_bindgen]
    pub fn step_frame(&mut self) -> Uint8Array {
        let buf = self.nes.tick_frame();
        return Uint8Array::from(buf);
    }
}

/// Installs a global panic handler to make debugging easier
#[wasm_bindgen]
pub fn init_debug_hooks() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}
