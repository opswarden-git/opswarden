use super::{automation_catalog, localize_capability, localize_field, resolve_client_ip};
use axum::http::{HeaderMap, HeaderValue};
use std::net::IpAddr;

#[test]
fn client_ip_only_trusts_the_configured_proxy_depth() {
    let peer: IpAddr = "10.0.0.8".parse().unwrap();
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-forwarded-for",
        HeaderValue::from_static("203.0.113.42, 10.0.0.7"),
    );

    assert_eq!(resolve_client_ip(peer, &headers, 0), peer);
    assert_eq!(
        resolve_client_ip(peer, &headers, 1),
        "10.0.0.7".parse::<IpAddr>().unwrap()
    );
    assert_eq!(
        resolve_client_ip(peer, &headers, 2),
        "203.0.113.42".parse::<IpAddr>().unwrap()
    );
}

#[test]
fn malformed_or_short_proxy_chains_fall_back_to_the_peer() {
    let peer: IpAddr = "10.0.0.8".parse().unwrap();
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-forwarded-for",
        HeaderValue::from_static("attacker, 10.0.0.7"),
    );
    assert_eq!(resolve_client_ip(peer, &headers, 1), peer);

    headers.insert("x-forwarded-for", HeaderValue::from_static("203.0.113.42"));
    assert_eq!(resolve_client_ip(peer, &headers, 2), peer);
}

#[test]
fn ipv4_mapped_addresses_are_exposed_in_canonical_ipv4_form() {
    let peer: IpAddr = "::ffff:203.0.113.42".parse().unwrap();
    assert_eq!(
        resolve_client_ip(peer, &HeaderMap::new(), 0),
        "203.0.113.42".parse::<IpAddr>().unwrap()
    );
}

#[test]
fn every_server_owned_catalog_sentence_has_a_french_variant() {
    let english = automation_catalog("en");
    let french = automation_catalog("fr");
    for (english_service, french_service) in english.iter().zip(&french) {
        for (english_item, french_item) in english_service
            .actions
            .iter()
            .chain(&english_service.reactions)
            .zip(
                french_service
                    .actions
                    .iter()
                    .chain(&french_service.reactions),
            )
        {
            assert_ne!(
                english_item.label, french_item.label,
                "{}",
                english_item.name
            );
            assert_ne!(
                english_item.description, french_item.description,
                "{}",
                english_item.name
            );
            for (english_field, french_field) in english_item.fields.iter().zip(&french_item.fields)
            {
                if !matches!(english_field.name.as_str(), "workflow" | "message") {
                    assert_ne!(
                        english_field.label, french_field.label,
                        "{}",
                        english_field.name
                    );
                }
                assert_ne!(
                    english_field.description, french_field.description,
                    "{}",
                    english_field.name
                );
                for (english_option, french_option) in
                    english_field.options.iter().zip(&french_field.options)
                {
                    assert_eq!(english_option.value, french_option.value);
                    assert_ne!(english_option.label, french_option.label);
                }
            }
        }
        if let (Some(english_connection), Some(french_connection)) =
            (&english_service.connection, &french_service.connection)
        {
            assert_ne!(
                english_connection.description, french_connection.description,
                "{}",
                english_service.name
            );
            for (english_field, french_field) in english_connection
                .fields
                .iter()
                .zip(&french_connection.fields)
            {
                assert_ne!(
                    english_field.label, french_field.label,
                    "{}",
                    english_field.name
                );
                assert_ne!(
                    english_field.description, french_field.description,
                    "{}",
                    english_field.name
                );
            }
            if let (Some(english_oauth), Some(french_oauth)) =
                (&english_connection.oauth, &french_connection.oauth)
            {
                assert_ne!(english_oauth.label, french_oauth.label);
                assert_ne!(english_oauth.description, french_oauth.description);
            }
        }
    }
}

#[test]
fn catalog_localization_is_keyed_by_stable_identifiers() {
    assert_eq!(
        localize_capability("gitlab", "ci_failed", "fr", "rewritten English", true),
        "Échec d’une pipeline CI"
    );
    assert_eq!(
        localize_field(
            "gitlab",
            "connection",
            "webhook_signing_secret",
            "fr",
            "rewritten English",
            false,
        ),
        "Obligatoire à la première connexion ; envoyé par GitLab dans X-Gitlab-Token"
    );
}
