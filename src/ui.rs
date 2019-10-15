//! UI framework code for deFeNEStrate
//!
//! This is shared between the WASM and native targets

use crate::devices::nes::NesEmulator;
use quicksilver::{
    geom::{Rectangle, Transform},
    graphics::{
        Background::{Col, Img},
        Color, Font, FontStyle, Image, PixelFormat,
    },
    input::{ButtonState, Key},
    lifecycle::{Asset, State, Window},
};

pub struct MainWindow {
    pub nes: NesEmulator,
    font: Asset<Font>,
    is_suspended: bool,
    status: String,
    use_pallete: bool,
    frame: Option<Image>,
}

impl State for MainWindow {
    fn new() -> quicksilver::Result<MainWindow> {
        let font = Asset::new(Font::load("DroidSansMono.ttf"));
        Ok(MainWindow {
            nes: NesEmulator::default(),
            font,
            is_suspended: true,
            use_pallete: false,
            status: String::from("<RESET>"),
            frame: Option::None,
        })
    }

    fn update(&mut self, window: &mut Window) -> quicksilver::Result<()> {
        let keyboard = window.keyboard();
        if keyboard[Key::F8] == ButtonState::Pressed {
            self.is_suspended = !self.is_suspended;
        }
        if keyboard[Key::F7] == ButtonState::Pressed {
            self.use_pallete = !self.use_pallete;
        }
        if !self.is_suspended {
            let frame = self.nes.run_frame();
            let img = Image::from_raw(&*frame, 256, 240, PixelFormat::RGB);
            self.frame = match img {
                quicksilver::Result::Ok(img) => Option::Some(img),
                quicksilver::Result::Err(_) => Option::None,
            };
            self.status = self.nes.get_status();
        } else if keyboard[Key::F9] == ButtonState::Pressed {
            self.status = self.nes.step_debug();
        } else if keyboard[Key::F10] == ButtonState::Pressed {
            let frame = self.nes.run_frame();
            let img = Image::from_raw(&*frame, 256, 240, PixelFormat::RGB);
            self.frame = match img {
                quicksilver::Result::Ok(img) => Option::Some(img),
                quicksilver::Result::Err(_) => Option::None,
            };
            self.status = self.nes.get_status();
        }
        Ok(())
    }

    fn draw(&mut self, window: &mut Window) -> quicksilver::Result<()> {
        window.clear(Color::WHITE)?;
        // Rendering area
        window.draw(&Rectangle::new((10, 10), (512, 480)), Col(Color::BLACK));
        if let Option::Some(frame) = &self.frame {
            window.draw_ex(
                &frame.area(),
                Img(&frame),
                Transform::scale((2, 2)) * Transform::translate((64 + 5, 60 + 5)),
                0,
            );
        }
        // Debugging
        let status = self.nes.get_status();
        let img = Image::from_raw(
            &*self.nes.get_chr(self.use_pallete),
            128,
            256,
            PixelFormat::RGB,
        )?;
        let pallete = Image::from_raw(&self.nes.get_palletes(), 128, 2, PixelFormat::RGB)?;
        let img_rect = &img.area();
        let pallete_rect = &pallete.area();
        window.draw(&Rectangle::new((660, 10), img_rect.size), Img(&img));
        window.draw(
            &Rectangle::new((660, 266), pallete_rect.size),
            Img(&pallete),
        );
        self.font.execute(|font| {
            let style = FontStyle::new(16.0, Color::BLACK);
            let img = font.render(&status, &style)?;
            let rect = &img.area();
            window.draw(&Rectangle::new((0, 500), rect.size), Img(&img));
            Ok(())
        })
    }
}
