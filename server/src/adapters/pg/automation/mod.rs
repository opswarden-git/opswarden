pub mod execution;
pub mod rule;
pub mod service_connection;

#[cfg(test)]
pub(super) mod test_support {
    use sqlx::PgPool;
    use uuid::Uuid;

    use crate::adapters::pg::team::PgTeamRepo;
    use crate::adapters::pg::user::PgUserRepo;
    use crate::domain::team::Team;
    use crate::domain::user::{Email, User};
    use crate::ports::{TeamRepo, UserRepo};

    pub async fn seed_team(pool: &PgPool, suffix: &str) -> (Uuid, Uuid) {
        let users = PgUserRepo::new(pool.clone());
        let teams = PgTeamRepo::new(pool.clone());
        let email =
            Email::new(format!("automation-{suffix}-{}@test.local", Uuid::new_v4())).unwrap();
        let user = User::new(email, "hash");
        users.save(&user).await.unwrap();

        let team = Team::new(format!("Automation {suffix}")).unwrap();
        teams.save_team(&team).await.unwrap();
        (team.id, user.id)
    }
}
