use sqlx::postgres::PgRow;
use sqlx::types::Json;
use sqlx::{PgPool, Row};

use super::StoreError;
use crate::{GamePage, PageSection, Slug};

const COLUMNS: &str =
    "slug, tagline, description, tags, accent, cover_image, skin, sections, featured, sort_order";

#[derive(Clone)]
pub struct PageRepo {
    pool: PgPool,
}

impl PageRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Creates the page of a game or replaces every field of the existing one. Pages are
    /// not versioned: editing one never touches the game's rules.
    pub async fn upsert(&self, page: &GamePage) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO game_pages \
                 (slug, tagline, description, tags, accent, cover_image, skin, sections, featured, sort_order) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
             ON CONFLICT (slug) DO UPDATE SET \
                 tagline = EXCLUDED.tagline, description = EXCLUDED.description, \
                 tags = EXCLUDED.tags, accent = EXCLUDED.accent, \
                 cover_image = EXCLUDED.cover_image, skin = EXCLUDED.skin, \
                 sections = EXCLUDED.sections, featured = EXCLUDED.featured, \
                 sort_order = EXCLUDED.sort_order",
        )
        .bind(page.slug.as_str())
        .bind(&page.tagline)
        .bind(&page.description)
        .bind(&page.tags)
        .bind(&page.accent)
        .bind(&page.cover_image)
        .bind(&page.skin)
        .bind(Json(&page.sections))
        .bind(page.featured)
        .bind(page.sort_order)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get(&self, slug: &Slug) -> Result<Option<GamePage>, StoreError> {
        let row = sqlx::query(&format!("SELECT {COLUMNS} FROM game_pages WHERE slug = $1"))
            .bind(slug.as_str())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(page_from_row).transpose()
    }

    /// Every page, by `sort_order` and then slug.
    pub async fn list(&self) -> Result<Vec<GamePage>, StoreError> {
        let rows = sqlx::query(&format!(
            "SELECT {COLUMNS} FROM game_pages ORDER BY sort_order, slug"
        ))
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(page_from_row).collect()
    }
}

fn page_from_row(row: &PgRow) -> Result<GamePage, StoreError> {
    Ok(GamePage {
        slug: Slug::parse(&row.try_get::<String, _>("slug")?)?,
        tagline: row.try_get("tagline")?,
        description: row.try_get("description")?,
        tags: row.try_get("tags")?,
        accent: row.try_get("accent")?,
        cover_image: row.try_get("cover_image")?,
        skin: row.try_get("skin")?,
        sections: row.try_get::<Json<Vec<PageSection>>, _>("sections")?.0,
        featured: row.try_get("featured")?,
        sort_order: row.try_get("sort_order")?,
    })
}
