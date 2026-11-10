use sqlx::postgres::PgRow;
use sqlx::types::Json;
use sqlx::{PgConnection, PgPool, Row};

use super::StoreError;
use crate::{GamePage, PageSection, Slug, Variant};

const COLUMNS: &str = "slug, name, preview_variant, tagline, description, tags, accent, \
                       cover_image, skin, sections, featured, sort_order";

#[derive(Clone)]
pub struct PageRepo {
    pool: PgPool,
}

impl PageRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Creates the page of a game or replaces every field of the existing one, except that a
    /// page without a cover keeps the cover it has: the seed saves pages at every start and
    /// knows nothing of uploaded covers. Use [`set_cover_image`](Self::set_cover_image) to
    /// change or clear one. Pages are not versioned: editing one never touches the game's rules.
    ///
    /// This saves the page alone. A game is listed only with variants: see
    /// [`save_game`](Self::save_game).
    pub async fn upsert(&self, page: &GamePage) -> Result<(), StoreError> {
        let mut conn = self.pool.acquire().await?;
        upsert_page(&mut conn, page).await
    }

    /// Saves a game and its variants in one transaction.
    ///
    /// Each variant is created, or replaced field by field; one that belongs to another game
    /// moves to this one. Then a page row that is now a variant of this game and has no
    /// variants of its own left is removed, which is how a game that used to be several pages
    /// (Go on three boards) becomes one. Nothing else is ever deleted: a variant the caller
    /// stops listing stays where it is.
    pub async fn save_game(&self, page: &GamePage, variants: &[Variant]) -> Result<(), StoreError> {
        let mut tx = self.pool.begin().await?;
        upsert_page(&mut tx, page).await?;
        for variant in variants {
            sqlx::query(
                "INSERT INTO game_variants (slug, game_slug, label, sort_order, skin) \
                 VALUES ($1, $2, $3, $4, $5) \
                 ON CONFLICT (slug) DO UPDATE SET \
                     game_slug = EXCLUDED.game_slug, label = EXCLUDED.label, \
                     sort_order = EXCLUDED.sort_order, skin = EXCLUDED.skin",
            )
            .bind(variant.slug.as_str())
            .bind(variant.game.as_str())
            .bind(&variant.label)
            .bind(variant.sort_order)
            .bind(&variant.skin)
            .execute(&mut *tx)
            .await?;
        }
        let slugs: Vec<String> = variants
            .iter()
            .map(|v| v.slug.as_str().to_string())
            .collect();
        sqlx::query(
            "DELETE FROM game_pages p \
             WHERE p.slug = ANY($1) AND p.slug <> $2 \
               AND NOT EXISTS (SELECT 1 FROM game_variants v WHERE v.game_slug = p.slug)",
        )
        .bind(&slugs)
        .bind(page.slug.as_str())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Sets, or with `None` clears, a game's cover: the object key of its image in the store.
    /// Returns whether the game has a page.
    pub async fn set_cover_image(
        &self,
        slug: &Slug,
        key: Option<&str>,
    ) -> Result<bool, StoreError> {
        let updated = sqlx::query("UPDATE game_pages SET cover_image = $2 WHERE slug = $1")
            .bind(slug.as_str())
            .bind(key)
            .execute(&self.pool)
            .await?;
        Ok(updated.rows_affected() > 0)
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

async fn upsert_page(conn: &mut PgConnection, page: &GamePage) -> Result<(), StoreError> {
    sqlx::query(
        "INSERT INTO game_pages \
             (slug, name, preview_variant, tagline, description, tags, accent, cover_image, \
              skin, sections, featured, sort_order) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) \
         ON CONFLICT (slug) DO UPDATE SET \
             name = EXCLUDED.name, preview_variant = EXCLUDED.preview_variant, \
             tagline = EXCLUDED.tagline, description = EXCLUDED.description, \
             tags = EXCLUDED.tags, accent = EXCLUDED.accent, \
             cover_image = COALESCE(EXCLUDED.cover_image, game_pages.cover_image), \
             skin = EXCLUDED.skin, sections = EXCLUDED.sections, \
             featured = EXCLUDED.featured, sort_order = EXCLUDED.sort_order",
    )
    .bind(page.slug.as_str())
    .bind(&page.name)
    .bind(page.preview_variant.as_ref().map(Slug::as_str))
    .bind(&page.tagline)
    .bind(&page.description)
    .bind(&page.tags)
    .bind(&page.accent)
    .bind(&page.cover_image)
    .bind(&page.skin)
    .bind(Json(&page.sections))
    .bind(page.featured)
    .bind(page.sort_order)
    .execute(conn)
    .await?;
    Ok(())
}

fn page_from_row(row: &PgRow) -> Result<GamePage, StoreError> {
    Ok(GamePage {
        slug: Slug::parse(&row.try_get::<String, _>("slug")?)?,
        name: row.try_get("name")?,
        preview_variant: row
            .try_get::<Option<String>, _>("preview_variant")?
            .map(|raw| Slug::parse(&raw))
            .transpose()?,
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
