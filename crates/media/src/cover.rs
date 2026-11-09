use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{ExtendedColorType, ImageFormat, ImageReader, Limits};
use sha2::{Digest, Sha256};

use crate::MediaError;

/// Largest file accepted, checked before anything is decoded.
pub const MAX_COVER_BYTES: usize = 10 * 1024 * 1024;
/// Longest side accepted, checked from the header before the picture is decoded.
pub const MAX_COVER_SIDE: u32 = 8000;
/// Width a stored cover is shrunk to; smaller pictures are left as they are.
pub const MAX_COVER_WIDTH: u32 = 1200;

const JPEG_QUALITY: u8 = 82;
/// Memory the decoder may use for one picture.
const MAX_DECODE_BYTES: u64 = 256 * 1024 * 1024;
const HASH_CHARS: usize = 16;

/// A cover ready to be stored: a fresh JPEG made here.
#[derive(Debug, Clone)]
pub struct ProcessedCover {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub content_type: &'static str,
}

/// Checks an uploaded picture and re-encodes it as a JPEG no wider than [`MAX_COVER_WIDTH`].
///
/// Only PNG, JPEG and WebP are accepted, by what the bytes are and not by any name. Size and
/// dimensions are checked before decoding, and the result is a new file, so metadata and
/// anything appended to the original never reach the store.
pub fn process_cover(input: &[u8]) -> Result<ProcessedCover, MediaError> {
    if input.len() > MAX_COVER_BYTES {
        return Err(MediaError::TooLarge {
            size: input.len(),
            limit: MAX_COVER_BYTES,
        });
    }
    let format = image::guess_format(input).map_err(|_| MediaError::UnsupportedFormat)?;
    if !matches!(
        format,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP
    ) {
        return Err(MediaError::UnsupportedFormat);
    }

    let (width, height) = ImageReader::with_format(Cursor::new(input), format)
        .into_dimensions()
        .map_err(|_| MediaError::Unreadable)?;
    if width > MAX_COVER_SIDE || height > MAX_COVER_SIDE {
        return Err(MediaError::TooManyPixels {
            width,
            height,
            limit: MAX_COVER_SIDE,
        });
    }

    let mut reader = ImageReader::with_format(Cursor::new(input), format);
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_COVER_SIDE);
    limits.max_image_height = Some(MAX_COVER_SIDE);
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    reader.limits(limits);
    let mut picture = reader.decode().map_err(|_| MediaError::Unreadable)?;

    if picture.width() > MAX_COVER_WIDTH {
        let scaled = (u64::from(picture.height()) * u64::from(MAX_COVER_WIDTH)
            + u64::from(picture.width()) / 2)
            / u64::from(picture.width());
        let scaled = u32::try_from(scaled).unwrap_or(u32::MAX).max(1);
        picture = picture.resize_exact(MAX_COVER_WIDTH, scaled, FilterType::Lanczos3);
    }

    // JPEG has no transparency; what lies under it is what shows.
    let rgb = picture.to_rgb8();
    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, JPEG_QUALITY)
        .encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            ExtendedColorType::Rgb8,
        )
        .map_err(|_| MediaError::Unreadable)?;

    Ok(ProcessedCover {
        bytes,
        width: rgb.width(),
        height: rgb.height(),
        content_type: "image/jpeg",
    })
}

/// Where a game's cover is kept: `covers/<game>/<hash of the picture>.jpg`. The name changes
/// whenever the picture does, so stored covers never need to be replaced or purged from a cache.
pub fn cover_key(slug: &str, cover: &ProcessedCover) -> Result<String, MediaError> {
    let valid = !slug.is_empty()
        && slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    if !valid {
        return Err(MediaError::InvalidSlug);
    }
    let digest = Sha256::digest(&cover.bytes);
    let hash: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(format!("covers/{slug}/{}.jpg", &hash[..HASH_CHARS]))
}
