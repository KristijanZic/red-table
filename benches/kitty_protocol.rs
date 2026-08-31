use std::{hint::black_box, time::Instant};

use image::{Rgb, RgbImage};
use ratatui::layout::Size;

#[allow(dead_code, unused_imports)]
#[path = "../source/kitty.rs"]
mod kitty;

use kitty::{KittyProtocol, KittySession};

const SOURCE_WIDTH: u32 = 640;
const SOURCE_HEIGHT: u32 = 360;
const CELL_WIDTH: u16 = 64;
const CELL_HEIGHT: u16 = 18;
const SAMPLES: usize = 7;

fn main() {
    let source = RgbImage::from_fn(SOURCE_WIDTH, SOURCE_HEIGHT, |x, y| {
        let detail = ((x.wrapping_mul(31) ^ y.wrapping_mul(17)) & 63) as u8;
        Rgb([
            (x * 255 / SOURCE_WIDTH) as u8,
            (y * 255 / SOURCE_HEIGHT) as u8,
            48_u8.saturating_add(detail),
        ])
    });
    let size = Size::new(CELL_WIDTH, CELL_HEIGHT);
    let session = KittySession::with_tmux(false);
    let mut samples = Vec::with_capacity(SAMPLES);

    for _ in 0..SAMPLES {
        let started = Instant::now();
        let protocol = KittyProtocol::new(
            black_box(source.clone().into()),
            size,
            black_box(session.clone()),
        )
        .unwrap();
        black_box(protocol.size());
        samples.push(started.elapsed());
    }
    samples.sort_unstable();

    println!(
        "Kitty RGBA transport: {SOURCE_WIDTH}x{SOURCE_HEIGHT} over {CELL_WIDTH}x{CELL_HEIGHT} cells, median of {SAMPLES}"
    );
    println!("prepare: {:.2?}", samples[SAMPLES / 2]);
}
