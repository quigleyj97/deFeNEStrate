//! UI framework code for deFeNEStrate
//!
//! This is shared between the WASM and native targets

use crate::devices::nes::NesEmulator;
use quicksilver::{
    geom::Rectangle,
    graphics::{
        Background::{Col, Img},
        Color, Font, FontStyle,
    },
    lifecycle::{Asset, State, Window},
};

pub struct MainWindow {
    pub nes: NesEmulator,
    font: Asset<Font>,
}

impl State for MainWindow {
    fn new() -> quicksilver::Result<MainWindow> {
        let font = Asset::new(Font::load("DroidSansMono.ttf"));
        Ok(MainWindow {
            nes: NesEmulator::default(),
            font,
        })
    }

    fn update(&mut self, _window: &mut Window) -> quicksilver::Result<()> {
        self.nes.step_emulator();
        Ok(())
    }

    fn draw(&mut self, window: &mut Window) -> quicksilver::Result<()> {
        window.clear(Color::WHITE)?;
        // Rendering area
        window.draw(&Rectangle::new((10, 10), (640, 480)), Col(Color::BLACK));
        // Debugging
        let status = self.nes.get_status();
        self.font.execute(|font| {
            let style = FontStyle::new(16.0, Color::BLACK);
            let img = font.render(&status, &style)?;
            let rect = &img.area();
            window.draw(&Rectangle::new((0, 500), rect.size), Img(&img));
            Ok(())
        })
    }
}
