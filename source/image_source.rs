use std::path::Path;

use image::{DynamicImage, ImageDecoder, Limits};

pub(crate) const MAX_DECODED_SOURCE_BYTES: u64 = 512 * 1024 * 1024;

pub(crate) fn decode_oriented(path: &Path) -> Result<DynamicImage, String> {
    let mut reader = image::ImageReader::open(path)
        .and_then(image::ImageReader::with_guessed_format)
        .map_err(|error| error.to_string())?;
    let mut limits = Limits::default();
    limits.max_alloc = Some(MAX_DECODED_SOURCE_BYTES);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(|error| error.to_string())?;
    if decoder.total_bytes() > MAX_DECODED_SOURCE_BYTES {
        return Err(format!(
            "decoded image exceeds the {} MiB source limit",
            MAX_DECODED_SOURCE_BYTES / (1024 * 1024)
        ));
    }
    let orientation = decoder.orientation().map_err(|error| error.to_string())?;
    let mut image = DynamicImage::from_decoder(decoder).map_err(|error| error.to_string())?;
    image.apply_orientation(orientation);
    Ok(image)
}

pub(crate) fn decoded_bytes(image: &DynamicImage) -> u64 {
    u64::from(image.width())
        .saturating_mul(u64::from(image.height()))
        .saturating_mul(u64::from(image.color().bytes_per_pixel()))
}

#[cfg(test)]
mod tests {
    use std::{fs::File, time::SystemTime};

    use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};

    use super::*;

    fn temporary_png() -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "red-table-oriented-{}-{nonce}.png",
            std::process::id()
        ))
    }

    #[test]
    fn applies_exif_orientation_during_decode() {
        let path = temporary_png();
        let mut encoder = PngEncoder::new(File::create(&path).unwrap());
        // Little-endian TIFF IFD with Orientation=6 (90 degrees clockwise).
        encoder
            .set_exif_metadata(vec![
                0x49, 0x49, 0x2a, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01, 0x00, 0x12, 0x01, 0x03, 0x00,
                0x01, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ])
            .unwrap();
        encoder
            .write_image(
                &[255, 0, 0, 255, 0, 0, 255, 255],
                2,
                1,
                ExtendedColorType::Rgba8,
            )
            .unwrap();

        let image = decode_oriented(&path).unwrap();
        assert_eq!((image.width(), image.height()), (1, 2));
        assert_eq!(decoded_bytes(&image), 8);
        std::fs::remove_file(path).unwrap();
    }
}
