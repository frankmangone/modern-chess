use std::collections::HashMap;

use modern_chess_kernel::UserId;
use sqlx::types::Json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::StoreError;
use crate::{result_for, Outcome, Record, SeatName, Slug};

/// Reads what people have done on games, from the matches that finished. A record is not stored:
/// it is counted when asked for, through [`result_for`], so there is one rule for how a match counts
/// and nothing to keep in step with it.
#[derive(Clone)]
pub struct RecordRepo {
    pool: PgPool,
}

/// One finished match, from one person's side.
struct Played {
    user: UserId,
    variant: Slug,
    seat: SeatName,
    outcome: Outcome,
}

impl RecordRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// The record of each of `users` on the game made of `variants`, all of them together. A person
    /// with no finished match is in the answer with an empty record.
    pub async fn of_game(
        &self,
        users: &[UserId],
        variants: &[Slug],
    ) -> Result<HashMap<UserId, Record>, StoreError> {
        let mut records: HashMap<UserId, Record> = users
            .iter()
            .map(|user| (*user, Record::default()))
            .collect();
        for played in self.finished(users, variants).await? {
            records
                .entry(played.user)
                .or_default()
                .add(result_for(&played.outcome, &played.seat));
        }
        Ok(records)
    }

    /// The record of one person in each variant they have finished a match in.
    pub async fn by_variant(
        &self,
        user: UserId,
        variants: &[Slug],
    ) -> Result<HashMap<Slug, Record>, StoreError> {
        let mut records: HashMap<Slug, Record> = HashMap::new();
        for played in self.finished(&[user], variants).await? {
            records
                .entry(played.variant)
                .or_default()
                .add(result_for(&played.outcome, &played.seat));
        }
        Ok(records)
    }

    /// The matches of `users` in any of `variants` that were finished with at least two people
    /// seated: a match nobody finished does not count, and neither does one played alone.
    async fn finished(
        &self,
        users: &[UserId],
        variants: &[Slug],
    ) -> Result<Vec<Played>, StoreError> {
        let rows = sqlx::query(
            "SELECT p.user_id, p.seat, i.slug, i.outcome \
             FROM game_players p \
             JOIN game_instances i ON i.id = p.instance_id \
             WHERE p.user_id = ANY($1) \
               AND i.slug = ANY($2) \
               AND i.status = 'FINISHED' \
               AND i.outcome IS NOT NULL \
               AND (SELECT count(*) FROM game_players o WHERE o.instance_id = i.id) >= 2",
        )
        .bind(users.iter().map(UserId::as_uuid).collect::<Vec<Uuid>>())
        .bind(
            variants
                .iter()
                .map(|v| v.as_str().to_string())
                .collect::<Vec<_>>(),
        )
        .fetch_all(&self.pool)
        .await?;

        let mut played = Vec::with_capacity(rows.len());
        for row in &rows {
            let outcome: Json<Outcome> = row.try_get("outcome")?;
            played.push(Played {
                user: UserId::from_uuid(row.try_get::<Uuid, _>("user_id")?),
                variant: Slug::parse(&row.try_get::<String, _>("slug")?)?,
                seat: SeatName::new(row.try_get::<String, _>("seat")?),
                outcome: outcome.0,
            });
        }
        Ok(played)
    }
}
