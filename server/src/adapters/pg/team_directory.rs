// --- server/src/adapters/pg/team_directory.rs ---
//
// The one read model behind the team switcher: every team a user belongs to,
// with the counts the interface ranks them by. It lives beside the repository
// because it is a single sixty-line statement, and a repository that carries it
// inline stops being readable as a list of what it can do.

use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::team::{InvitationCode, Team, TeamDirectoryItem};

use super::team_mapping::role_from_str;

pub(super) async fn list_for_user(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<TeamDirectoryItem>, DomainError> {
    let records = sqlx::query!(
        r#"
        SELECT
            t.id,
            t.name,
            t.invitation_code,
            t.created_at,
            image.updated_at AS "image_updated_at?",
            membership.role,
            (SELECT COUNT(*) FROM team_members members WHERE members.team_id = t.id) AS "member_count!",
            (SELECT COUNT(*) FROM incidents incidents
                WHERE incidents.team_id = t.id AND incidents.status <> 'resolved') AS "active_incident_count!",
            (SELECT COUNT(*) FROM releases releases
                WHERE releases.team_id = t.id
                  AND releases.base_state IN ('created', 'in_progress')) AS "active_release_count!",
            (SELECT COUNT(*) FROM releases releases
                WHERE releases.team_id = t.id
                  AND releases.base_state = 'in_progress'
                  AND EXISTS (
                      SELECT 1
                      FROM release_incidents links
                      JOIN incidents incidents ON incidents.id = links.incident_id
                      WHERE links.release_id = releases.id
                        AND incidents.status <> 'resolved'
                  )) AS "blocked_release_count!"
        FROM team_members membership
        JOIN teams t ON t.id = membership.team_id
        LEFT JOIN team_images image ON image.team_id = t.id
        WHERE membership.user_id = $1
        ORDER BY membership.joined_at
        "#,
        user_id,
    )
    .fetch_all(pool)
    .await
    .map_err(|_| DomainError::Storage)?;

    Ok(records
        .into_iter()
        .map(|row| TeamDirectoryItem {
            team: Team {
                id: row.id,
                name: row.name,
                invitation_code: InvitationCode::from_existing(row.invitation_code),
                created_at: row.created_at,
            },
            role: role_from_str(&row.role),
            member_count: row.member_count as u64,
            active_incident_count: row.active_incident_count as u64,
            active_release_count: row.active_release_count as u64,
            blocked_release_count: row.blocked_release_count as u64,
            image_updated_at: row.image_updated_at,
        })
        .collect())
}
