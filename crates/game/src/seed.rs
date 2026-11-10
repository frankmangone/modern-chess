//! Fills an empty (or older) database with the games that ship with the product.
//!
//! Rules come from the engine's spec files, pages from this crate's `pages/` directory.
//! Both are embedded at build time. Seeding is safe to repeat: identical rules change
//! nothing, changed rules become a new version, and a page is replaced field by field.
//!
//! A page file may list the variants of its game. Seeding only adds and moves: a variant a
//! file stops listing stays where it is. The one deletion is the page row of a game that has
//! become a variant of another (see [`PageRepo::save_game`]).

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use tracing::warn;

use crate::store::{PageRepo, SpecRepo, StoreError};
use crate::{GamePage, Slug, Variant};

include!(concat!(env!("OUT_DIR"), "/seed_files.rs"));

/// What a seeding run did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SeedReport {
    pub specs: usize,
    pub pages: usize,
    pub variants: usize,
    /// Files that were left out, each with the reason. A bad file never stops the others.
    pub skipped: Vec<String>,
}

/// A page file: the page itself, plus the variants of the game it describes.
#[derive(Deserialize)]
struct PageFile {
    #[serde(flatten)]
    page: GamePage,
    /// Empty means one variant: the game's own slug, labelled with its rules' name.
    #[serde(default)]
    variants: Vec<VariantFile>,
}

#[derive(Deserialize)]
struct VariantFile {
    slug: Slug,
    label: String,
    #[serde(default)]
    skin: Option<Value>,
}

/// Seeds the embedded files.
pub async fn run(
    specs: &SpecRepo,
    pages: &PageRepo,
    now: DateTime<Utc>,
) -> Result<SeedReport, StoreError> {
    run_with(specs, pages, SPEC_FILES, PAGE_FILES, now).await
}

/// Seeds the given `(file stem, JSON text)` pairs. A spec's slug is its file stem, with `_`
/// written as `-`; a page carries its own slug.
pub async fn run_with(
    specs: &SpecRepo,
    pages: &PageRepo,
    spec_files: &[(&str, &str)],
    page_files: &[(&str, &str)],
    now: DateTime<Utc>,
) -> Result<SeedReport, StoreError> {
    let mut report = SeedReport::default();

    for (stem, text) in spec_files {
        match read_spec(stem, text) {
            Ok((slug, name, rules)) => match specs.upsert(slug, name, rules, now).await {
                Ok(_) => report.specs += 1,
                Err(StoreError::Domain(err)) => skip(&mut report, format!("spec {stem}: {err}")),
                Err(other) => return Err(other),
            },
            Err(reason) => skip(&mut report, format!("spec {stem}: {reason}")),
        }
    }

    for (stem, text) in page_files {
        match serde_json::from_str::<PageFile>(text) {
            Ok(file) => match game_from_file(specs, file).await? {
                Ok((page, variants)) => {
                    pages.save_game(&page, &variants).await?;
                    report.pages += 1;
                    report.variants += variants.len();
                }
                Err(reason) => skip(&mut report, format!("page {stem}: {reason}")),
            },
            Err(err) => skip(&mut report, format!("page {stem}: {err}")),
        }
    }

    Ok(report)
}

/// The page and variants a file describes, or why it cannot be used. The outer error is the
/// database's; the inner one is the file's fault.
async fn game_from_file(
    specs: &SpecRepo,
    file: PageFile,
) -> Result<Result<(GamePage, Vec<Variant>), String>, StoreError> {
    let PageFile { mut page, variants } = file;
    let variants = if variants.is_empty() {
        let label = match specs.latest(&page.slug).await? {
            Some(spec) => spec.name,
            None => page.slug.as_str().to_string(),
        };
        vec![Variant::single(&page.slug, &label)]
    } else {
        let mut seen = Vec::new();
        for variant in &variants {
            if seen.contains(&&variant.slug) {
                return Ok(Err(format!("variant {} is listed twice", variant.slug)));
            }
            seen.push(&variant.slug);
        }
        variants
            .into_iter()
            .zip(0..)
            .map(|(variant, sort_order)| Variant {
                slug: variant.slug,
                game: page.slug.clone(),
                label: variant.label,
                sort_order,
                skin: variant.skin,
            })
            .collect()
    };

    if let Some(preview) = &page.preview_variant {
        if !variants.iter().any(|v| &v.slug == preview) {
            return Ok(Err(format!(
                "preview_variant {preview} is not one of its variants"
            )));
        }
    }
    if page.name.is_none() {
        let named = page
            .preview_variant
            .as_ref()
            .or(variants.first().map(|v| &v.slug));
        if let Some(named) = named {
            page.name = specs.latest(named).await?.map(|spec| spec.name);
        }
    }
    Ok(Ok((page, variants)))
}

fn skip(report: &mut SeedReport, reason: String) {
    warn!("seed file skipped: {reason}");
    report.skipped.push(reason);
}

fn read_spec(stem: &str, text: &str) -> Result<(Slug, String, serde_json::Value), String> {
    let slug = Slug::parse(&stem.replace('_', "-")).map_err(|err| err.to_string())?;
    let rules: serde_json::Value = serde_json::from_str(text).map_err(|err| err.to_string())?;
    let name = rules
        .get("name")
        .and_then(|name| name.as_str())
        .map(display_name)
        .ok_or_else(|| "the rules have no name".to_string())?;
    Ok((slug, name, rules))
}

/// `CAPABLANCA_CHESS` becomes `Capablanca Chess`.
fn display_name(engine_name: &str) -> String {
    engine_name
        .split(['_', ' '])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first
                    .to_uppercase()
                    .chain(chars.flat_map(char::to_lowercase))
                    .collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}
