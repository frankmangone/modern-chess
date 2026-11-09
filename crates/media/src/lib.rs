//! Game images: checking and re-encoding an uploaded picture, naming it, and keeping it in an
//! S3-compatible store. It knows nothing about games or users; callers pass names and bytes.
//!
//! Everything that comes in is treated as untrusted, because uploads will one day come from
//! players. A picture is never stored as it arrived: it is decoded and written again.

mod cover;
mod error;
mod store;

pub use cover::{
    cover_key, process_cover, ProcessedCover, MAX_COVER_BYTES, MAX_COVER_SIDE, MAX_COVER_WIDTH,
};
pub use error::MediaError;
pub use store::{public_read_policy, ImageStore, StoreConfig};
