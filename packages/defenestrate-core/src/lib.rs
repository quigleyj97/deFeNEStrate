#[macro_use]
extern crate bitflags;

extern crate wasm_bindgen;

use wasm_bindgen::prelude::*;

pub mod devices;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn hello(buf: &[u8]) {
    use crate::devices::cpu::WithCpu;
    use devices::nes::Nes;

    alert("Hello, world!");
    let mut nes = Nes::new_from_buf(buf);
    nes.cpu_mut().state.pc = 0xC000;

    let mut line = 1;

    for i in 0..5000 {
        alert(&format!("L{:04} {}", line, &nes.dbg_step_cpu()));
        line += 1;
    }
}
