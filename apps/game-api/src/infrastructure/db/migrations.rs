use std::collections::HashSet;

use sqlx::PgPool;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, Clone)]
pub struct SchemaStatus {
    pub latest_known_version: Option<i64>,
    pub latest_applied_version: Option<i64>,
    pub applied_count: usize,
    pub pending_versions: Vec<i64>,
}

impl SchemaStatus {
    pub fn is_current(&self) -> bool {
        self.pending_versions.is_empty()
    }
}

pub async fn run(pool: &PgPool) -> Result<SchemaStatus, sqlx::migrate::MigrateError> {
    MIGRATOR.run(pool).await?;
    status(pool).await
}

pub async fn status(pool: &PgPool) -> Result<SchemaStatus, sqlx::migrate::MigrateError> {
    let local_versions: Vec<i64> = MIGRATOR.iter().map(|m| m.version).collect();

    let table_exists: Option<String> =
        sqlx::query_scalar("SELECT to_regclass('_sqlx_migrations')::text")
            .fetch_one(pool)
            .await
            .map_err(sqlx::migrate::MigrateError::Execute)?;

    let applied_versions: Vec<i64> = if table_exists.is_some() {
        sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
            .fetch_all(pool)
            .await
            .map_err(sqlx::migrate::MigrateError::Execute)?
    } else {
        Vec::new()
    };

    let applied_set: HashSet<i64> = applied_versions.iter().copied().collect();
    let pending_versions = local_versions
        .iter()
        .copied()
        .filter(|version| !applied_set.contains(version))
        .collect::<Vec<_>>();

    Ok(SchemaStatus {
        latest_known_version: local_versions.iter().max().copied(),
        latest_applied_version: applied_versions.iter().max().copied(),
        applied_count: applied_versions.len(),
        pending_versions,
    })
}
