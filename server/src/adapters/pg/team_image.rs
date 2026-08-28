// --- server/src/adapters/pg/team_image.rs ---
//
// The three statements behind a team's image. They live beside the repository
// rather than inside its trait impl, which a single feature had grown past the
// size a reviewer can hold in their head.

use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::team::TeamImage;

pub(super) async fn save(
    pool: &PgPool,
    team_id: Uuid,
    image: &TeamImage,
) -> Result<(), DomainError> {
    sqlx::query!(
        r#"
        INSERT INTO team_images (team_id, media_type, content, updated_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (team_id) DO UPDATE SET
            media_type = EXCLUDED.media_type,
            content = EXCLUDED.content,
            updated_at = EXCLUDED.updated_at
        "#,
        team_id,
        image.media_type,
        image.content,
        image.updated_at,
    )
    .execute(pool)
    .await
    .map_err(|_| DomainError::Storage)?;
    Ok(())
}

/// Membership is checked in the statement itself: an image is scoped to a team,
/// so a non-member must not be able to read one by knowing its id.
pub(super) async fn find_for_member(
    pool: &PgPool,
    team_id: Uuid,
    user_id: Uuid,
) -> Result<Option<TeamImage>, DomainError> {
    let row = sqlx::query!(
        r#"
        SELECT image.media_type, image.content, image.updated_at
        FROM team_images image
        WHERE image.team_id = $1
          AND EXISTS (
              SELECT 1 FROM team_members member
              WHERE member.team_id = image.team_id AND member.user_id = $2
          )
        "#,
        team_id,
        user_id,
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| DomainError::Storage)?;
    Ok(row.map(|row| TeamImage {
        media_type: row.media_type,
        content: row.content,
        updated_at: row.updated_at,
    }))
}

pub(super) async fn delete(pool: &PgPool, team_id: Uuid) -> Result<(), DomainError> {
    sqlx::query!("DELETE FROM team_images WHERE team_id = $1", team_id)
        .execute(pool)
        .await
        .map_err(|_| DomainError::Storage)?;
    Ok(())
}
