//! Fills an empty (or older) database with the games that ship with the product.
//!
//! Rules come from the engine's spec files, pages from this crate's `pages/` directory.
//! Both are embedded at build time. Seeding is safe to repeat: identical rules change
//! nothing, changed rules become a new version, and a page is replaced field by field.

use chrono::{DateTime, Utc};
use tracing::warn;

use crate::store::{PageRepo, SpecRepo, StoreError};
use crate::{GamePage, Slug};

include!(concat!(env!("OUT_DIR"), "/seed_files.rs"));

/// What a seeding run did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SeedReport {
    pub specs: usize,
    pub pages: usize,
    /// Files that were left out, each with the reason. A bad file never stops the others.
    pub skipped: Vec<String>,
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
        match serde_json::from_str::<GamePage>(text) {
            Ok(page) => {
                pages.upsert(&page).await?;
                report.pages += 1;
            }
            Err(err) => skip(&mut report, format!("page {stem}: {err}")),
        }
    }

    Ok(report)
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
