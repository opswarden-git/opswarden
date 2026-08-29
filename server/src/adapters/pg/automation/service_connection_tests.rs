use super::super::test_support::seed_team;
use super::*;

const KEY: [u8; aes::KEY_LEN] = [73; aes::KEY_LEN];

/// `service_connection_secrets.kind` carries an allowlist constraint, so a
/// variant the migrations do not know about is rejected by Postgres and
/// surfaces as `storage_error`. The HTTP-level tests use an in-memory vault
/// that enforces nothing, which is how the Email vertical reached production
/// with five unusable credential kinds. This exercises every variant against
/// the real schema.
#[sqlx::test]
async fn every_credential_kind_round_trips_through_postgres(pool: PgPool) {
    let (team_id, manager_id) = seed_team(&pool, "credential-kinds").await;
    let connections = PgServiceConnectionRepo::new(pool.clone());
    let connection = ServiceConnection::new(team_id, "github", manager_id).unwrap();
    connections.insert_connection(&connection).await.unwrap();
    let vault = PgConnectionCredentialVault::new(pool, KEY);

    for kind in CredentialKind::ALL {
        let secret = format!("secret-for-{kind}");
        vault
            .store_credential(connection.id, *kind, &secret)
            .await
            .unwrap_or_else(|error| {
                panic!("storing {kind} failed with {error:?}; a migration is missing")
            });
        assert_eq!(
            vault.reveal_credential(connection.id, *kind).await.unwrap(),
            Some(secret)
        );
    }

    let configured = vault
        .configured_credential_kinds(connection.id)
        .await
        .unwrap();
    assert_eq!(configured.len(), CredentialKind::ALL.len());
}

#[sqlx::test]
async fn manager_membership_creates_one_credential_free_opswarden_connection(pool: PgPool) {
    let (team_id, manager_id) = seed_team(&pool, "native-opswarden").await;
    let repo = PgServiceConnectionRepo::new(pool.clone());

    let connection = repo
        .find_connection_by_service(team_id, "opswarden")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(connection.team_id, team_id);
    assert_eq!(connection.created_by, Some(manager_id));
    assert!(PgConnectionCredentialVault::new(pool, KEY)
        .configured_credential_kinds(connection.id)
        .await
        .unwrap()
        .is_empty());
}

#[sqlx::test]
async fn the_same_provider_is_isolated_between_teams(pool: PgPool) {
    let (team_a, user_a) = seed_team(&pool, "connections-a").await;
    let (team_b, user_b) = seed_team(&pool, "connections-b").await;
    let repo = PgServiceConnectionRepo::new(pool.clone());
    let github_a = ServiceConnection::new(team_a, "github", user_a).unwrap();
    let github_b = ServiceConnection::new(team_b, "github", user_b).unwrap();
    repo.insert_connection(&github_a).await.unwrap();
    repo.insert_connection(&github_b).await.unwrap();

    let external_for = |connections: Vec<ServiceConnection>| {
        connections
            .into_iter()
            .filter(|connection| connection.service == "github")
            .collect::<Vec<_>>()
    };
    assert_eq!(
        external_for(repo.list_connections_for_team(team_a).await.unwrap()),
        vec![github_a.clone()]
    );
    assert_eq!(
        external_for(repo.list_connections_for_team(team_b).await.unwrap()),
        vec![github_b.clone()]
    );
    assert!(repo
        .find_connection_for_team(team_a, github_b.id)
        .await
        .unwrap()
        .is_none());
}

#[sqlx::test]
async fn credentials_are_encrypted_and_separated_by_connection_and_kind(pool: PgPool) {
    let (team_a, user_a) = seed_team(&pool, "vault-a").await;
    let (team_b, user_b) = seed_team(&pool, "vault-b").await;
    let repo = PgServiceConnectionRepo::new(pool.clone());
    let github_a = ServiceConnection::new(team_a, "github", user_a).unwrap();
    let github_b = ServiceConnection::new(team_b, "github", user_b).unwrap();
    repo.insert_connection(&github_a).await.unwrap();
    repo.insert_connection(&github_b).await.unwrap();

    let vault = PgConnectionCredentialVault::new(pool.clone(), KEY);
    vault
        .store_credential(
            github_a.id,
            CredentialKind::WebhookSigningSecret,
            "team-a-signing-secret",
        )
        .await
        .unwrap();
    vault
        .store_credential(
            github_a.id,
            CredentialKind::PersonalToken,
            "github_pat_team_a",
        )
        .await
        .unwrap();
    vault
        .store_credential(
            github_a.id,
            CredentialKind::OAuthAccessToken,
            "github_oauth_access_team_a",
        )
        .await
        .unwrap();
    vault
        .store_credential(
            github_a.id,
            CredentialKind::OAuthRefreshToken,
            "github_oauth_refresh_team_a",
        )
        .await
        .unwrap();
    vault
        .store_credential(
            github_b.id,
            CredentialKind::WebhookSigningSecret,
            "team-b-signing-secret",
        )
        .await
        .unwrap();

    assert_eq!(
        vault
            .reveal_credential(github_a.id, CredentialKind::WebhookSigningSecret)
            .await
            .unwrap()
            .as_deref(),
        Some("team-a-signing-secret")
    );
    assert_eq!(
        vault
            .reveal_credential(github_b.id, CredentialKind::WebhookSigningSecret)
            .await
            .unwrap()
            .as_deref(),
        Some("team-b-signing-secret")
    );
    assert_eq!(
        vault
            .reveal_credential(github_a.id, CredentialKind::OAuthAccessToken)
            .await
            .unwrap()
            .as_deref(),
        Some("github_oauth_access_team_a")
    );
    assert_eq!(
        vault
            .reveal_credential(github_a.id, CredentialKind::OAuthRefreshToken)
            .await
            .unwrap()
            .as_deref(),
        Some("github_oauth_refresh_team_a")
    );

    let rows =
        sqlx::query("SELECT ciphertext FROM service_connection_secrets WHERE connection_id = $1")
            .bind(github_a.id)
            .fetch_all(&pool)
            .await
            .unwrap();
    let plaintexts: &[&[u8]] = &[
        b"github_pat_team_a",
        b"github_oauth_access_team_a",
        b"github_oauth_refresh_team_a",
        b"team-a-signing-secret",
    ];
    for row in rows {
        let ciphertext: Vec<u8> = row.try_get("ciphertext").unwrap();
        assert!(plaintexts
            .iter()
            .all(|plaintext| ciphertext.as_slice() != *plaintext));
    }

    assert_eq!(
        vault
            .configured_credential_kinds(github_a.id)
            .await
            .unwrap(),
        vec![
            CredentialKind::OAuthAccessToken,
            CredentialKind::OAuthRefreshToken,
            CredentialKind::PersonalToken,
            CredentialKind::WebhookSigningSecret
        ]
    );
    let touched_connection = repo
        .find_connection_for_team(team_a, github_a.id)
        .await
        .unwrap()
        .unwrap();
    assert!(touched_connection.updated_at >= github_a.updated_at);
}

#[sqlx::test]
async fn deleting_connection_cascades_credentials_but_is_team_scoped(pool: PgPool) {
    let (team_a, user_a) = seed_team(&pool, "delete-a").await;
    let (team_b, user_b) = seed_team(&pool, "delete-b").await;
    let repo = PgServiceConnectionRepo::new(pool.clone());
    let connection = ServiceConnection::new(team_a, "github", user_a).unwrap();
    let other = ServiceConnection::new(team_b, "github", user_b).unwrap();
    repo.insert_connection(&connection).await.unwrap();
    repo.insert_connection(&other).await.unwrap();
    let vault = PgConnectionCredentialVault::new(pool, KEY);
    vault
        .store_credential(
            connection.id,
            CredentialKind::WebhookSigningSecret,
            "secret",
        )
        .await
        .unwrap();
    vault
        .store_credential(
            connection.id,
            CredentialKind::OAuthAccessToken,
            "oauth-access",
        )
        .await
        .unwrap();
    vault
        .store_credential(
            connection.id,
            CredentialKind::OAuthRefreshToken,
            "oauth-refresh",
        )
        .await
        .unwrap();

    assert!(!repo.delete_connection(team_b, connection.id).await.unwrap());
    assert!(vault
        .reveal_credential(connection.id, CredentialKind::WebhookSigningSecret)
        .await
        .unwrap()
        .is_some());
    assert!(repo.delete_connection(team_a, connection.id).await.unwrap());
    assert!(vault
        .reveal_credential(connection.id, CredentialKind::WebhookSigningSecret)
        .await
        .unwrap()
        .is_none());
    assert!(vault
        .reveal_credential(connection.id, CredentialKind::OAuthAccessToken)
        .await
        .unwrap()
        .is_none());
    assert!(vault
        .reveal_credential(connection.id, CredentialKind::OAuthRefreshToken)
        .await
        .unwrap()
        .is_none());
}

#[sqlx::test]
async fn signed_delivery_health_is_persisted_without_resetting_first_verification(pool: PgPool) {
    let (team_id, user_id) = seed_team(&pool, "delivery-health").await;
    let repo = PgServiceConnectionRepo::new(pool);
    let connection = ServiceConnection::new(team_id, "github", user_id).unwrap();
    repo.insert_connection(&connection).await.unwrap();

    repo.record_delivery_result(connection.id, None)
        .await
        .unwrap();
    let verified = repo
        .find_connection_for_team(team_id, connection.id)
        .await
        .unwrap()
        .unwrap();
    assert!(verified.verified_at.is_some());
    assert!(verified.last_delivery_at.is_some());
    assert_eq!(verified.last_error_code, None);

    repo.record_delivery_result(connection.id, Some("invalid_automation_rule"))
        .await
        .unwrap();
    let failed = repo
        .find_connection_for_team(team_id, connection.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(failed.verified_at, verified.verified_at);
    assert!(failed.last_delivery_at >= verified.last_delivery_at);
    assert_eq!(
        failed.last_error_code.as_deref(),
        Some("invalid_automation_rule")
    );
}

#[sqlx::test]
async fn outbound_health_does_not_claim_an_inbound_delivery_and_resets_on_replace(pool: PgPool) {
    let (team_id, user_id) = seed_team(&pool, "reaction-health").await;
    let repo = PgServiceConnectionRepo::new(pool);
    let connection = ServiceConnection::new(team_id, "http", user_id).unwrap();
    repo.insert_connection(&connection).await.unwrap();

    repo.record_reaction_result(connection.id, None)
        .await
        .unwrap();
    let verified = repo
        .find_connection_for_team(team_id, connection.id)
        .await
        .unwrap()
        .unwrap();
    assert!(verified.verified_at.is_some());
    assert!(verified.last_delivery_at.is_none());

    repo.record_reaction_result(connection.id, Some("reaction_http_5xx"))
        .await
        .unwrap();
    let failed = repo
        .find_connection_for_team(team_id, connection.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(failed.last_error_code.as_deref(), Some("reaction_http_5xx"));

    repo.reset_connection_health(connection.id).await.unwrap();
    let reset = repo
        .find_connection_for_team(team_id, connection.id)
        .await
        .unwrap()
        .unwrap();
    assert!(reset.verified_at.is_none());
    assert!(reset.last_error_code.is_none());
    assert!(reset.last_delivery_at.is_none());
}
