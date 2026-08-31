// PostgreSQL-backed Team automation rules.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::domain::automation_catalog::TIMER_SERVICE;
use crate::domain::automation_config::AutomationRule;
use crate::domain::automation_timer::TimerSchedule;
use crate::domain::error::DomainError;
use crate::ports::AutomationRuleRepo;

pub struct PgAutomationRuleRepo {
    pool: PgPool,
}

impl PgAutomationRuleRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

async fn connection_service(
    transaction: &mut Transaction<'_, Postgres>,
    team_id: Uuid,
    connection_id: Uuid,
) -> Result<Option<String>, DomainError> {
    sqlx::query_scalar("SELECT service FROM service_connections WHERE team_id = $1 AND id = $2")
        .bind(team_id)
        .bind(connection_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(|_| DomainError::Storage)
}

async fn insert_timer_projection(
    transaction: &mut Transaction<'_, Postgres>,
    rule: &AutomationRule,
    schedule: &TimerSchedule,
    next_run_at: DateTime<Utc>,
) -> Result<(), DomainError> {
    let (local_time, interval_minutes) = match schedule {
        TimerSchedule::DailyAt { time, .. } => (Some(*time), None),
        TimerSchedule::EveryMinutes { minutes, .. } => (None, Some(i32::from(*minutes))),
    };
    sqlx::query(
        r#"
        INSERT INTO automation_timer_schedules (
            rule_id, schedule_kind, timezone, local_time, interval_minutes,
            next_run_at, rule_updated_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, now())
        ON CONFLICT (rule_id) DO UPDATE
        SET schedule_kind = excluded.schedule_kind,
            timezone = excluded.timezone,
            local_time = excluded.local_time,
            interval_minutes = excluded.interval_minutes,
            next_run_at = excluded.next_run_at,
            rule_updated_at = excluded.rule_updated_at,
            last_claimed_at = null,
            updated_at = now()
        "#,
    )
    .bind(rule.id)
    .bind(schedule.kind())
    .bind(schedule.timezone().to_string())
    .bind(local_time)
    .bind(interval_minutes)
    .bind(next_run_at)
    .bind(rule.updated_at)
    .execute(&mut **transaction)
    .await
    .map_err(|_| DomainError::Storage)?;
    Ok(())
}

#[derive(FromRow)]
struct AutomationRuleRow {
    id: Uuid,
    team_id: Uuid,
    name: String,
    enabled: bool,
    trigger_connection_id: Uuid,
    trigger_kind: String,
    trigger_config: Value,
    reaction_kind: String,
    reaction_connection_id: Option<Uuid>,
    reaction_config: Value,
    created_by: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<AutomationRuleRow> for AutomationRule {
    fn from(row: AutomationRuleRow) -> Self {
        Self {
            id: row.id,
            team_id: row.team_id,
            name: row.name,
            enabled: row.enabled,
            trigger_connection_id: row.trigger_connection_id,
            trigger_kind: row.trigger_kind,
            trigger_config: row.trigger_config,
            reaction_kind: row.reaction_kind,
            reaction_connection_id: row.reaction_connection_id,
            reaction_config: row.reaction_config,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[async_trait]
impl AutomationRuleRepo for PgAutomationRuleRepo {
    async fn insert_rule(&self, rule: &AutomationRule) -> Result<(), DomainError> {
        let mut transaction = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        let service =
            connection_service(&mut transaction, rule.team_id, rule.trigger_connection_id).await?;
        let timer_schedule = if service.as_deref() == Some(TIMER_SERVICE) {
            Some(TimerSchedule::from_config(
                &rule.trigger_kind,
                &rule.trigger_config,
            )?)
        } else {
            None
        };
        sqlx::query(
            r#"
            INSERT INTO automation_rules (
                id, team_id, name, enabled, trigger_connection_id, trigger_kind,
                trigger_config, reaction_kind, reaction_connection_id,
                reaction_config, created_by, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
        )
        .bind(rule.id)
        .bind(rule.team_id)
        .bind(&rule.name)
        .bind(rule.enabled)
        .bind(rule.trigger_connection_id)
        .bind(&rule.trigger_kind)
        .bind(&rule.trigger_config)
        .bind(&rule.reaction_kind)
        .bind(rule.reaction_connection_id)
        .bind(&rule.reaction_config)
        .bind(rule.created_by)
        .bind(rule.created_at)
        .bind(rule.updated_at)
        .execute(&mut *transaction)
        .await
        .map_err(|_| DomainError::Storage)?;
        if rule.enabled {
            if let Some(schedule) = timer_schedule {
                let next_run_at = schedule.next_after(rule.updated_at);
                insert_timer_projection(&mut transaction, rule, &schedule, next_run_at).await?;
            }
        }
        transaction
            .commit()
            .await
            .map_err(|_| DomainError::Storage)?;
        Ok(())
    }

    async fn update_rule(
        &self,
        rule: &AutomationRule,
        expected_updated_at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        let mut transaction = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        let previous = sqlx::query_as::<_, (bool, Uuid, String, Value, DateTime<Utc>)>(
            r#"
            SELECT enabled, trigger_connection_id, trigger_kind, trigger_config, updated_at
            FROM automation_rules
            WHERE team_id = $1 AND id = $2
            FOR UPDATE
            "#,
        )
        .bind(rule.team_id)
        .bind(rule.id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| DomainError::Storage)?;
        let Some(previous) = previous else {
            transaction
                .commit()
                .await
                .map_err(|_| DomainError::Storage)?;
            return Ok(false);
        };
        if previous.4 != expected_updated_at {
            return Err(DomainError::ConcurrentModification);
        }
        let service =
            connection_service(&mut transaction, rule.team_id, rule.trigger_connection_id).await?;
        let timer_schedule = if service.as_deref() == Some(TIMER_SERVICE) {
            Some(TimerSchedule::from_config(
                &rule.trigger_kind,
                &rule.trigger_config,
            )?)
        } else {
            None
        };
        let result = sqlx::query(
            r#"
            UPDATE automation_rules
            SET name = $3,
                enabled = $4,
                trigger_connection_id = $5,
                trigger_kind = $6,
                trigger_config = $7,
                reaction_kind = $8,
                reaction_connection_id = $9,
                reaction_config = $10,
                updated_at = $11
            WHERE team_id = $1 AND id = $2
            "#,
        )
        .bind(rule.team_id)
        .bind(rule.id)
        .bind(&rule.name)
        .bind(rule.enabled)
        .bind(rule.trigger_connection_id)
        .bind(&rule.trigger_kind)
        .bind(&rule.trigger_config)
        .bind(&rule.reaction_kind)
        .bind(rule.reaction_connection_id)
        .bind(&rule.reaction_config)
        .bind(rule.updated_at)
        .execute(&mut *transaction)
        .await
        .map_err(|_| DomainError::Storage)?;
        if !rule.enabled || timer_schedule.is_none() {
            sqlx::query("DELETE FROM automation_timer_schedules WHERE rule_id = $1")
                .bind(rule.id)
                .execute(&mut *transaction)
                .await
                .map_err(|_| DomainError::Storage)?;
        } else if let Some(schedule) = timer_schedule {
            let schedule_changed = !previous.0
                || previous.1 != rule.trigger_connection_id
                || previous.2 != rule.trigger_kind
                || previous.3 != rule.trigger_config;
            if schedule_changed {
                let next_run_at = schedule.next_after(rule.updated_at);
                insert_timer_projection(&mut transaction, rule, &schedule, next_run_at).await?;
            } else {
                sqlx::query(
                    r#"
                    UPDATE automation_timer_schedules
                    SET rule_updated_at = $2, updated_at = now()
                    WHERE rule_id = $1
                    "#,
                )
                .bind(rule.id)
                .bind(rule.updated_at)
                .execute(&mut *transaction)
                .await
                .map_err(|_| DomainError::Storage)?;
            }
        }
        transaction
            .commit()
            .await
            .map_err(|_| DomainError::Storage)?;
        Ok(result.rows_affected() == 1)
    }

    async fn find_rule_for_team(
        &self,
        team_id: Uuid,
        rule_id: Uuid,
    ) -> Result<Option<AutomationRule>, DomainError> {
        let row = sqlx::query_as::<_, AutomationRuleRow>(
            r#"
            SELECT id, team_id, name, enabled, trigger_connection_id,
                   trigger_kind, trigger_config, reaction_kind,
                   reaction_connection_id, reaction_config, created_by,
                   created_at, updated_at
            FROM automation_rules
            WHERE team_id = $1 AND id = $2
            "#,
        )
        .bind(team_id)
        .bind(rule_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        Ok(row.map(Into::into))
    }

    async fn list_rules_for_team(&self, team_id: Uuid) -> Result<Vec<AutomationRule>, DomainError> {
        Ok(sqlx::query_as::<_, AutomationRuleRow>(
            r#"
            SELECT id, team_id, name, enabled, trigger_connection_id,
                   trigger_kind, trigger_config, reaction_kind,
                   reaction_connection_id, reaction_config, created_by,
                   created_at, updated_at
            FROM automation_rules
            WHERE team_id = $1
            ORDER BY created_at, id
            "#,
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?
        .into_iter()
        .map(Into::into)
        .collect())
    }

    async fn list_enabled_rules_for_trigger(
        &self,
        team_id: Uuid,
        connection_id: Uuid,
        trigger_kind: &str,
    ) -> Result<Vec<AutomationRule>, DomainError> {
        Ok(sqlx::query_as::<_, AutomationRuleRow>(
            r#"
            SELECT id, team_id, name, enabled, trigger_connection_id,
                   trigger_kind, trigger_config, reaction_kind,
                   reaction_connection_id, reaction_config, created_by,
                   created_at, updated_at
            FROM automation_rules
            WHERE team_id = $1
              AND trigger_connection_id = $2
              AND trigger_kind = $3
              AND enabled
            ORDER BY created_at, id
            "#,
        )
        .bind(team_id)
        .bind(connection_id)
        .bind(trigger_kind)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?
        .into_iter()
        .map(Into::into)
        .collect())
    }

    async fn delete_rule(&self, team_id: Uuid, rule_id: Uuid) -> Result<bool, DomainError> {
        let result = sqlx::query("DELETE FROM automation_rules WHERE team_id = $1 AND id = $2")
            .bind(team_id)
            .bind(rule_id)
            .execute(&self.pool)
            .await
            .map_err(|_| DomainError::Storage)?;
        Ok(result.rows_affected() == 1)
    }

    async fn next_run_at(
        &self,
        team_id: Uuid,
        rule_id: Uuid,
    ) -> Result<Option<DateTime<Utc>>, DomainError> {
        sqlx::query_scalar(
            r#"
            SELECT s.next_run_at
            FROM automation_timer_schedules s
            JOIN automation_rules r ON r.id = s.rule_id
            WHERE r.team_id = $1 AND r.id = $2 AND r.enabled
            "#,
        )
        .bind(team_id)
        .bind(rule_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::super::service_connection::PgServiceConnectionRepo;
    use super::super::test_support::seed_team;
    use super::*;
    use crate::domain::automation_config::ServiceConnection;
    use crate::ports::ServiceConnectionRepo;

    async fn connection(
        repo: &PgServiceConnectionRepo,
        team_id: Uuid,
        user_id: Uuid,
        service: &str,
    ) -> ServiceConnection {
        let connection = ServiceConnection::new(team_id, service, user_id).unwrap();
        repo.insert_connection(&connection).await.unwrap();
        connection
    }

    fn rule(team_id: Uuid, user_id: Uuid, trigger: Uuid) -> AutomationRule {
        AutomationRule::new(
            team_id,
            "CI failed -> incident",
            trigger,
            "ci_failed",
            json!({"repository": "opswarden/app"}),
            "create_incident",
            None,
            json!({"title": "CI failed", "severity": "high"}),
            user_id,
        )
        .unwrap()
    }

    #[sqlx::test]
    async fn rules_are_persisted_updated_and_listed_only_for_their_team(pool: PgPool) {
        let (team_a, user_a) = seed_team(&pool, "rules-a").await;
        let (team_b, user_b) = seed_team(&pool, "rules-b").await;
        let connections = PgServiceConnectionRepo::new(pool.clone());
        let github_a = connection(&connections, team_a, user_a, "github").await;
        let _github_b = connection(&connections, team_b, user_b, "github").await;
        let repo = PgAutomationRuleRepo::new(pool);
        let mut stored = rule(team_a, user_a, github_a.id);
        repo.insert_rule(&stored).await.unwrap();

        assert_eq!(repo.list_rules_for_team(team_b).await.unwrap(), vec![]);
        assert_eq!(
            repo.list_rules_for_team(team_a).await.unwrap(),
            vec![stored.clone()]
        );
        assert_eq!(
            repo.list_enabled_rules_for_trigger(team_a, github_a.id, "ci_failed")
                .await
                .unwrap(),
            vec![]
        );

        let expected_updated_at = stored.updated_at;
        stored.set_enabled(true);
        assert!(repo
            .update_rule(&stored, expected_updated_at)
            .await
            .unwrap());
        assert_eq!(
            repo.list_enabled_rules_for_trigger(team_a, github_a.id, "ci_failed")
                .await
                .unwrap(),
            vec![stored]
        );
    }

    #[sqlx::test]
    async fn database_rejects_cross_team_trigger_and_reaction_connections(pool: PgPool) {
        let (team_a, user_a) = seed_team(&pool, "cross-rule-a").await;
        let (team_b, user_b) = seed_team(&pool, "cross-rule-b").await;
        let connections = PgServiceConnectionRepo::new(pool.clone());
        let github_a = connection(&connections, team_a, user_a, "github").await;
        let github_b = connection(&connections, team_b, user_b, "github").await;
        let http_b = connection(&connections, team_b, user_b, "http").await;
        let repo = PgAutomationRuleRepo::new(pool);

        let foreign_trigger = rule(team_a, user_a, github_b.id);
        assert_eq!(
            repo.insert_rule(&foreign_trigger).await.unwrap_err(),
            DomainError::Storage
        );

        let mut foreign_reaction = rule(team_a, user_a, github_a.id);
        foreign_reaction.name = "Foreign HTTP reaction".to_string();
        foreign_reaction.reaction_kind = "http_post".to_string();
        foreign_reaction.reaction_connection_id = Some(http_b.id);
        assert_eq!(
            repo.insert_rule(&foreign_reaction).await.unwrap_err(),
            DomainError::Storage
        );
    }

    #[sqlx::test]
    async fn deleting_connection_referenced_by_rule_is_restricted(pool: PgPool) {
        let (team_id, user_id) = seed_team(&pool, "rule-delete").await;
        let connections = PgServiceConnectionRepo::new(pool.clone());
        let github = connection(&connections, team_id, user_id, "github").await;
        let rules = PgAutomationRuleRepo::new(pool);
        rules
            .insert_rule(&rule(team_id, user_id, github.id))
            .await
            .unwrap();

        assert_eq!(
            connections
                .delete_connection(team_id, github.id)
                .await
                .unwrap_err(),
            DomainError::Storage
        );
    }
}
