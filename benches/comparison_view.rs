use std::{hint::black_box, thread, time::Instant};

use image::{DynamicImage, GenericImageView, Rgb, RgbImage, imageops::FilterType};

const SOURCE_WIDTH: u32 = 6000;
const SOURCE_HEIGHT: u32 = 4000;
const PANE_WIDTH: u32 = 640;
const PANE_HEIGHT: u32 = 720;
const SAMPLES: usize = 7;

fn main() {
    let source = representative_image();
    println!(
        "A/B preparation: two {SOURCE_WIDTH}x{SOURCE_HEIGHT} sources -> {PANE_WIDTH}x{PANE_HEIGHT} panes, median of {SAMPLES}"
    );
    measure("paired warm fit", || {
        thread::scope(|scope| {
            let left = scope.spawn(|| source.resize(PANE_WIDTH, PANE_HEIGHT, FilterType::Lanczos3));
            let right =
                scope.spawn(|| source.resize(PANE_WIDTH, PANE_HEIGHT, FilterType::Lanczos3));
            let left = left.join().unwrap();
            let right = right.join().unwrap();
            black_box(left.get_pixel(PANE_WIDTH / 2, PANE_HEIGHT / 2));
            black_box(right.get_pixel(PANE_WIDTH / 2, PANE_HEIGHT / 2));
        });
    });
    measure("paired warm 100% crop", || {
        thread::scope(|scope| {
            let prepare = || {
                source
                    .crop_imm(
                        (SOURCE_WIDTH - PANE_WIDTH) / 2,
                        (SOURCE_HEIGHT - PANE_HEIGHT) / 2,
                        PANE_WIDTH,
                        PANE_HEIGHT,
                    )
                    .resize_exact(PANE_WIDTH, PANE_HEIGHT, FilterType::Lanczos3)
            };
            let left = scope.spawn(prepare);
            let right = scope.spawn(prepare);
            black_box(left.join().unwrap().get_pixel(0, 0));
            black_box(right.join().unwrap().get_pixel(0, 0));
        });
    });
}

fn measure(label: &str, operation: impl Fn()) {
    operation();
    let mut samples = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let started = Instant::now();
        operation();
        samples.push(started.elapsed());
    }
    samples.sort_unstable();
    println!("{label}: {:.2?}", samples[SAMPLES / 2]);
}

fn representative_image() -> DynamicImage {
    RgbImage::from_fn(SOURCE_WIDTH, SOURCE_HEIGHT, |x, y| {
        let grain = ((x.wrapping_mul(17) ^ y.wrapping_mul(29)) & 31) as u8;
        let edge = if (x / 48 + y / 48).is_multiple_of(2) {
            48
        } else {
            0
        };
        Rgb([
            (x * 255 / SOURCE_WIDTH) as u8,
            (y * 255 / SOURCE_HEIGHT) as u8,
            64_u8.saturating_add(edge).saturating_add(grain),
        ])
    })
    .into()
}
