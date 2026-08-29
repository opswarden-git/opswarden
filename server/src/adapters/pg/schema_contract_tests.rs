use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Display;

use sqlx::PgPool;

use crate::domain::incident::{IncidentStatus, Severity};
use crate::domain::release::ReleaseBaseState;
use crate::domain::team::Role;

fn stored_values<T: Display>(values: &[T]) -> BTreeSet<String> {
    values.iter().map(ToString::to_string).collect()
}

fn quoted_values(constraint: &str) -> BTreeSet<String> {
    constraint
        .split('\'')
        .enumerate()
        .filter_map(|(index, value)| (index % 2 == 1).then(|| value.to_owned()))
        .collect()
}

#[sqlx::test]
async fn persisted_domain_enums_match_postgres_allowlists(pool: PgPool) {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "select conname, pg_get_constraintdef(oid) from pg_constraint \
         where connamespace = current_schema()::regnamespace and conname = any($1)",
    )
    .bind(
        &[
            "team_members_role_check",
            "incidents_status_check",
            "incidents_severity_check",
            "releases_base_state_check",
        ][..],
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let actual: BTreeMap<_, _> = rows
        .into_iter()
        .map(|(name, definition)| (name, quoted_values(&definition)))
        .collect();
    let expected = BTreeMap::from([
        ("team_members_role_check".into(), stored_values(Role::ALL)),
        (
            "incidents_status_check".into(),
            stored_values(IncidentStatus::ALL),
        ),
        (
            "incidents_severity_check".into(),
            stored_values(Severity::ALL),
        ),
        (
            "releases_base_state_check".into(),
            stored_values(ReleaseBaseState::ALL),
        ),
    ]);

    assert_eq!(
        actual, expected,
        "Postgres allowlists diverge from the domain"
    );
}
