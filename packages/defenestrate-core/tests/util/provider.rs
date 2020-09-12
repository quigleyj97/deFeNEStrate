use std::fs::File;
use std::io::{BufRead, BufReader};
use std::iter::Iterator;
use std::path::Path;

const NESTEST_GOLD_LOG_PATH: &str = "./tests/data/nestest.log";
pub const NESTEST_ROM_PATH: &str = "./tests/data/nestest.nes";

pub fn load_gold_standard_log() -> impl Iterator<Item = String> {
    let path = Path::new(NESTEST_GOLD_LOG_PATH);
    let file = File::open(path).expect("Failed to read NESTEST gold log");
    let file = BufReader::new(file);
    file.lines().map(|line| String::from(line.unwrap().trim()))
}
