/// Turns what the database keeps for an image (an object key such as
/// `covers/chess/3f9a….jpg`) into an address a browser can load.
///
/// The database never holds a host, so moving the store (MinIO, S3, a CDN) means changing one
/// setting and no rows. A stored value is always a key under the base: it is never taken as an
/// address of its own, because one day players will supply them.
#[derive(Clone, Default)]
pub struct ImageUrls {
    base: Option<String>,
}

impl ImageUrls {
    pub fn new(base: &str) -> Self {
        Self {
            base: Some(base.trim_end_matches('/').to_string()),
        }
    }

    /// The address of `stored`. With no base configured the value is passed on unchanged.
    pub fn url(&self, stored: Option<String>) -> Option<String> {
        let stored = stored?;
        Some(match &self.base {
            Some(base) => format!("{base}/{}", stored.trim_start_matches('/')),
            None => stored,
        })
    }
}
