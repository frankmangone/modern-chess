use thiserror::Error;

/// Why an image was refused or could not be kept. The messages are safe to show to the person
/// who uploaded it; none of them quotes the file or the decoder.
#[derive(Debug, Error)]
pub enum MediaError {
    #[error("the file is not a PNG, JPEG or WebP picture")]
    UnsupportedFormat,
    #[error("the file is {size} bytes; the limit is {limit}")]
    TooLarge { size: usize, limit: usize },
    #[error("the picture is {width} by {height} pixels; no side may be over {limit}")]
    TooManyPixels { width: u32, height: u32, limit: u32 },
    #[error("the picture could not be read")]
    Unreadable,
    #[error("a game name may only use lowercase letters, digits and hyphens")]
    InvalidSlug,
    #[error("a public folder must be lowercase letters, digits and hyphens, ending in a slash")]
    InvalidFolder,
    #[error("image storage failed: {0}")]
    Storage(String),
}
