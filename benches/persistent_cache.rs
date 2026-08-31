use std::{
    fs,
    hint::black_box,
    path::PathBuf,
    time::{Instant, SystemTime},
};

use image::{Rgb, RgbImage};

#[allow(dead_code, unused_imports)]
#[path = "../source/image_source.rs"]
mod image_source;

#[allow(dead_code, unused_imports)]
#[path = "../source/persistent_cache.rs"]
mod persistent_cache;

use persistent_cache::PersistentCache;

const SAMPLES: usize = 7;
const SOURCE_WIDTH: u32 = 1920;
const SOURCE_HEIGHT: u32 = 1080;
const REQUIRED_PIXELS: (u32, u32) = (320, 180);

fn main() {
    let directory = temporary_directory();
    fs::create_dir_all(&directory).unwrap();
    let source = directory.join("representative.png");
    representative_image().save(&source).unwrap();

    let mut cold = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        let cache = PersistentCache::with_root(directory.join(format!("cold-{sample}/thumbnails")));
        let started = Instant::now();
        let prepared = cache.prepare(&source, REQUIRED_PIXELS).unwrap();
        black_box(prepared.image.width());
        assert!(!prepared.disk_hit);
        cold.push(started.elapsed());
    }

    let cache = PersistentCache::with_root(directory.join("warm/thumbnails"));
    cache.prepare(&source, REQUIRED_PIXELS).unwrap();
    let mut warm = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let started = Instant::now();
        let prepared = cache.prepare(&source, REQUIRED_PIXELS).unwrap();
        black_box(prepared.image.width());
        assert!(prepared.disk_hit);
        warm.push(started.elapsed());
    }

    cold.sort_unstable();
    warm.sort_unstable();
    println!(
        "persistent cache: {SOURCE_WIDTH}x{SOURCE_HEIGHT}, required {}x{}, median of {SAMPLES}",
        REQUIRED_PIXELS.0, REQUIRED_PIXELS.1
    );
    println!("cold original decode + write: {:.2?}", cold[SAMPLES / 2]);
    println!("warm standard PNG hit: {:.2?}", warm[SAMPLES / 2]);

    fs::remove_dir_all(directory).unwrap();
}

fn representative_image() -> RgbImage {
    RgbImage::from_fn(SOURCE_WIDTH, SOURCE_HEIGHT, |x, y| {
        let detail = ((x.wrapping_mul(31) ^ y.wrapping_mul(17)) & 63) as u8;
        Rgb([
            (x * 255 / SOURCE_WIDTH) as u8,
            (y * 255 / SOURCE_HEIGHT) as u8,
            48_u8.saturating_add(detail),
        ])
    })
}

fn temporary_directory() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "red-table-cache-benchmark-{}-{nonce}",
        std::process::id()
    ))
}
