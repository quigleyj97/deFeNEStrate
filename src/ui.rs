//! UI framework code for deFeNEStrate
//!
//! This is shared between the WASM and native targets

use crate::devices::nes::NesEmulator;
use quicksilver::{
    geom::{Circle, Line, Rectangle, Transform, Triangle},
    graphics::{Background::Col, Color},
    lifecycle::{State, Window},
};

pub struct MainWindow {
    pub nes: NesEmulator,
}

impl State for MainWindow {
    fn new() -> quicksilver::Result<MainWindow> {
        Ok(MainWindow {
            nes: NesEmulator::default(),
        })
    }

    fn draw(&mut self, window: &mut Window) -> quicksilver::Result<()> {
        window.clear(Color::WHITE)?;
        window.draw(&Rectangle::new((100, 100), (32, 32)), Col(Color::BLUE));
        window.draw_ex(
            &Rectangle::new((400, 300), (32, 32)),
            Col(Color::BLUE),
            Transform::rotate(45),
            10,
        );
        window.draw(&Circle::new((400, 300), 100), Col(Color::GREEN));
        window.draw_ex(
            &Line::new((50, 80), (600, 450)).with_thickness(2.0),
            Col(Color::RED),
            Transform::IDENTITY,
            5,
        );
        window.draw_ex(
            &Triangle::new((500, 50), (450, 100), (650, 150)),
            Col(Color::RED),
            Transform::rotate(45) * Transform::scale((0.5, 0.5)),
            0,
        );
        Ok(())
    }
}
