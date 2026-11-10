use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};

use super::StoreError;
use crate::{Slug, Variant};

const COLUMNS: &str = "slug, game_slug, label, sort_order, skin";

/// Reads the variants of games. They are written together with their game, by
/// [`PageRepo::save_game`](super::PageRepo::save_game).
#[derive(Clone)]
pub struct VariantRepo {
    pool: PgPool,
}

impl VariantRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get(&self, slug: &Slug) -> Result<Option<Variant>, StoreError> {
        let row = sqlx::query(&format!(
            "SELECT {COLUMNS} FROM game_variants WHERE slug = $1"
        ))
        .bind(slug.as_str())
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(variant_from_row).transpose()
    }

    /// The variants of one game, by `sort_order` and then slug.
    pub async fn for_game(&self, game: &Slug) -> Result<Vec<Variant>, StoreError> {
        let rows = sqlx::query(&format!(
            "SELECT {COLUMNS} FROM game_variants WHERE game_slug = $1 ORDER BY sort_order, slug"
        ))
        .bind(game.as_str())
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(variant_from_row).collect()
    }

    /// Every variant of every game, by `sort_order` and then slug.
    pub async fn all(&self) -> Result<Vec<Variant>, StoreError> {
        let rows = sqlx::query(&format!(
            "SELECT {COLUMNS} FROM game_variants ORDER BY sort_order, slug"
        ))
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(variant_from_row).collect()
    }
}

fn variant_from_row(row: &PgRow) -> Result<Variant, StoreError> {
    Ok(Variant {
        slug: Slug::parse(&row.try_get::<String, _>("slug")?)?,
        game: Slug::parse(&row.try_get::<String, _>("game_slug")?)?,
        label: row.try_get("label")?,
        sort_order: row.try_get("sort_order")?,
        skin: row.try_get("skin")?,
    })
}
