//! What happens to an uploaded cover before it is stored: only real, reasonably sized pictures
//! get through, and what is stored is a fresh JPEG made here, never the bytes that were sent.

use std::io::Cursor;

use image::{ImageFormat, Rgb, RgbImage, Rgba, RgbaImage};
use modern_chess_media::{
    cover_key, process_cover, MediaError, MAX_COVER_BYTES, MAX_COVER_SIDE, MAX_COVER_WIDTH,
};

fn encoded(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
    let picture = RgbImage::from_fn(width, height, |x, y| {
        Rgb([(x % 255) as u8, (y % 255) as u8, 90])
    });
    let mut bytes = Vec::new();
    picture
        .write_to(&mut Cursor::new(&mut bytes), format)
        .unwrap();
    bytes
}

fn png(width: u32, height: u32) -> Vec<u8> {
    encoded(width, height, ImageFormat::Png)
}

#[test]
fn a_large_picture_is_shrunk_to_the_cover_width_and_keeps_its_shape() {
    let cover = process_cover(&png(2400, 1350)).unwrap();

    assert_eq!((cover.width, cover.height), (MAX_COVER_WIDTH, 675));
    assert_eq!(cover.content_type, "image/jpeg");
    assert_eq!(&cover.bytes[..2], &[0xFF, 0xD8], "a JPEG starts with FF D8");
    let decoded = image::load_from_memory(&cover.bytes).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (MAX_COVER_WIDTH, 675));
}

#[test]
fn a_small_picture_is_not_stretched() {
    let cover = process_cover(&png(600, 300)).unwrap();

    assert_eq!((cover.width, cover.height), (600, 300));
}

#[test]
fn a_picture_with_transparency_is_accepted() {
    let picture = RgbaImage::from_pixel(64, 64, Rgba([200, 10, 10, 128]));
    let mut bytes = Vec::new();
    picture
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .unwrap();

    assert!(process_cover(&bytes).is_ok());
}

#[test]
fn jpeg_and_webp_pictures_are_accepted_too() {
    for format in [ImageFormat::Jpeg, ImageFormat::WebP] {
        let cover = process_cover(&encoded(400, 200, format)).unwrap();

        assert_eq!((cover.width, cover.height), (400, 200), "{format:?}");
    }
}

#[test]
fn text_is_not_a_picture() {
    let result = process_cover(b"just some words, not an image");

    assert!(matches!(result, Err(MediaError::UnsupportedFormat)));
}

#[test]
fn only_png_jpeg_and_webp_are_accepted() {
    let gif = b"GIF89a\x01\x00\x01\x00\x00\x00\x00;";

    assert!(matches!(
        process_cover(gif),
        Err(MediaError::UnsupportedFormat)
    ));
}

#[test]
fn a_file_over_the_size_limit_is_refused_without_decoding_it() {
    let huge = vec![0_u8; MAX_COVER_BYTES + 1];

    assert!(matches!(
        process_cover(&huge),
        Err(MediaError::TooLarge { .. })
    ));
}

#[test]
fn a_picture_with_an_absurd_side_is_refused_from_its_header_alone() {
    // Tiny on disk, but 8001 pixels wide: decoding such files is how memory gets exhausted.
    let result = process_cover(&png(MAX_COVER_SIDE + 1, 1));

    assert!(matches!(result, Err(MediaError::TooManyPixels { .. })));
}

#[test]
fn what_is_stored_is_re_encoded_so_hidden_extras_do_not_travel() {
    let mut bytes = png(200, 100);
    bytes.extend_from_slice(b"HIDDEN-TRAILER-DATA");

    let cover = process_cover(&bytes).unwrap();

    let text = String::from_utf8_lossy(&cover.bytes);
    assert!(!text.contains("HIDDEN-TRAILER-DATA"));
}

#[test]
fn the_key_names_the_game_and_the_content() {
    let cover = process_cover(&png(300, 200)).unwrap();
    let same = process_cover(&png(300, 200)).unwrap();
    let other = process_cover(&png(301, 200)).unwrap();

    let key = cover_key("chess", &cover).unwrap();

    assert!(
        key.starts_with("covers/chess/") && key.ends_with(".jpg"),
        "{key}"
    );
    let hash = key
        .trim_start_matches("covers/chess/")
        .trim_end_matches(".jpg");
    assert_eq!(hash.len(), 16);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(
        key,
        cover_key("chess", &same).unwrap(),
        "same picture, same key"
    );
    assert_ne!(
        key,
        cover_key("chess", &other).unwrap(),
        "other picture, other key"
    );
}

#[test]
fn a_game_name_cannot_steer_the_key_out_of_its_folder() {
    let cover = process_cover(&png(10, 10)).unwrap();

    for bad in ["", "../secrets", "Chess", "a/b", "chess ", "ches\u{0}s"] {
        assert!(
            matches!(cover_key(bad, &cover), Err(MediaError::InvalidSlug)),
            "{bad:?} must be refused"
        );
    }
    assert!(cover_key("four-player-chess", &cover).is_ok());
}
