// --- server/src/handlers/mod.rs ---

use std::net::{IpAddr, SocketAddr};

use axum::{
    extract::{ConnectInfo, Query, State},
    http::{header::HeaderName, HeaderMap},
    Json,
};
use serde::{Deserialize, Serialize};

pub mod auth;
pub mod error;
pub mod gif;
pub mod incident;
pub mod middleware;
pub mod private_message;
pub mod release;
pub mod team;
pub mod team_automation;
pub mod webhook;
pub mod ws;

#[derive(Serialize)]
pub struct Health {
    pub status: &'static str,
}

pub async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

#[derive(Serialize)]
pub struct About {
    pub client: ClientInfo,
    pub server: ServerInfo,
}

#[derive(Serialize)]
pub struct ClientInfo {
    pub host: String,
}

#[derive(Serialize)]
pub struct ServerInfo {
    pub current_time: u64,
    pub token: String,
    pub services: Vec<ServiceCatalog>,
}

#[derive(Serialize)]
pub struct ServiceCatalog {
    pub name: String,
    pub label: String,
    pub actions: Vec<CatalogItem>,
    pub reactions: Vec<CatalogItem>,
    pub connection: Option<ConnectionCatalog>,
}

#[derive(Serialize)]
pub struct CatalogItem {
    pub name: String,
    pub label: String,
    pub description: String,
    pub connection_service: Option<String>,
    pub fields: Vec<CatalogField>,
}

#[derive(Serialize)]
pub struct CatalogField {
    pub name: String,
    pub label: String,
    pub description: String,
    pub input_type: String,
    pub required: bool,
    pub default_value: Option<String>,
    pub options: Vec<CatalogOption>,
}

#[derive(Serialize)]
pub struct CatalogOption {
    pub value: String,
    pub label: String,
}

#[derive(Serialize)]
pub struct ConnectionCatalog {
    pub description: String,
    pub fields: Vec<CatalogField>,
    pub oauth: Option<OAuthCatalog>,
    pub testable: bool,
}

#[derive(Serialize)]
pub struct OAuthCatalog {
    pub label: String,
    pub description: String,
}

#[derive(Default, Deserialize)]
pub struct AboutQuery {
    pub locale: Option<String>,
}

use crate::AppState;

pub async fn about(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Query(query): Query<AboutQuery>,
    headers: HeaderMap,
) -> Json<About> {
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    Json(About {
        client: ClientInfo {
            host: resolve_client_ip(peer.ip(), &headers, state.config.trusted_proxy_hops)
                .to_string(),
        },
        server: ServerInfo {
            current_time,
            token: state.config.kickoff_token(),
            services: automation_catalog(if query.locale.as_deref() == Some("fr") {
                "fr"
            } else {
                "en"
            }),
        },
    })
}

fn canonical_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(ipv6) => ipv6
            .to_ipv4_mapped()
            .map(IpAddr::V4)
            .unwrap_or(IpAddr::V6(ipv6)),
        ipv4 => ipv4,
    }
}

fn resolve_client_ip(peer: IpAddr, headers: &HeaderMap, trusted_proxy_hops: usize) -> IpAddr {
    let peer = canonical_ip(peer);
    if trusted_proxy_hops == 0 {
        return peer;
    }

    let forwarded_for = HeaderName::from_static("x-forwarded-for");
    let Some(value) = headers
        .get(forwarded_for)
        .and_then(|value| value.to_str().ok())
    else {
        return peer;
    };
    let addresses = value
        .split(',')
        .map(|part| part.trim().parse::<IpAddr>().map(canonical_ip))
        .collect::<Result<Vec<_>, _>>();
    match addresses {
        Ok(addresses) if addresses.len() >= trusted_proxy_hops => {
            addresses[addresses.len() - trusted_proxy_hops]
        }
        _ => peer,
    }
}

/// The Action -> REAction catalog the engine actually supports, surfaced on
/// `/about.json` so the contract is server-driven (nothing hard-coded client
/// side). Grows as services/Actions/REActions are added in `adapters/webhook`
/// and the rule engine.
fn automation_catalog(locale: &str) -> Vec<ServiceCatalog> {
    crate::domain::automation_catalog::AUTOMATION_CATALOG
        .iter()
        .map(|service| ServiceCatalog {
            name: service.service.to_string(),
            label: service.label.to_string(),
            actions: service
                .actions
                .iter()
                .map(|item| catalog_item(item, locale))
                .collect(),
            reactions: service
                .reactions
                .iter()
                .map(|item| catalog_item(item, locale))
                .collect(),
            connection: service.connection.map(|connection| ConnectionCatalog {
                description: localize_connection(service.service, locale, connection.description),
                fields: connection
                    .fields
                    .iter()
                    .map(|field| catalog_field(field, locale))
                    .collect(),
                oauth: connection.oauth.map(|oauth| OAuthCatalog {
                    label: localize_oauth(service.service, locale, oauth.label, true),
                    description: localize_oauth(service.service, locale, oauth.description, false),
                }),
                testable: connection.testable,
            }),
        })
        .collect()
}

fn catalog_item(
    item: &crate::domain::automation_catalog::CatalogCapability,
    locale: &str,
) -> CatalogItem {
    CatalogItem {
        name: item.kind.to_string(),
        label: localize_capability(item.kind, locale, item.label, true),
        description: localize_capability(item.kind, locale, item.description, false),
        connection_service: item.connection_service.map(str::to_string),
        fields: item
            .fields
            .iter()
            .map(|field| catalog_field(field, locale))
            .collect(),
    }
}

fn catalog_field(
    field: &crate::domain::automation_catalog::CatalogField,
    locale: &str,
) -> CatalogField {
    CatalogField {
        name: field.name.to_string(),
        label: localize_field(field.name, locale, field.label, true),
        description: localize_field(field.name, locale, field.description, false),
        input_type: field.input_type.to_string(),
        required: field.required,
        default_value: field.default_value.map(str::to_string),
        options: field
            .options
            .iter()
            .map(|option| CatalogOption {
                value: option.to_string(),
                label: localize_option(option, locale),
            })
            .collect(),
    }
}

fn localize_capability(kind: &str, locale: &str, fallback: &str, label: bool) -> String {
    if locale != "fr" {
        return fallback.to_string();
    }
    match (kind, label, fallback) {
        ("ci_failed", true, "Pipeline failed") => "Échec d’une pipeline CI",
        ("ci_failed", false, "A GitLab CI/CD pipeline completed with a failing status") => {
            "Une pipeline GitLab CI/CD s’est terminée avec un statut en échec"
        }
        ("ci_failed", true, _) => "Échec d’un workflow CI",
        ("ci_failed", false, _) => {
            "Un workflow GitHub Actions s’est terminé avec un résultat en échec"
        }
        ("ci_succeeded", true, "Pipeline succeeded") => "Succès d’une pipeline CI",
        ("ci_succeeded", false, "A GitLab CI/CD pipeline completed successfully") => {
            "Une pipeline GitLab CI/CD s’est terminée avec succès"
        }
        ("ci_succeeded", true, _) => "Succès d’un workflow CI",
        ("ci_succeeded", false, _) => "Un workflow GitHub Actions s’est terminé avec succès",
        ("tag_pushed", true, _) => "Nouveau tag poussé",
        ("tag_pushed", false, "A new Git tag was pushed to the GitLab project") => {
            "Un nouveau tag Git a été poussé dans le projet GitLab"
        }
        ("tag_pushed", false, _) => "Un nouveau tag Git a été poussé dans le dépôt",
        ("pr_merged", true, _) => "Pull request fusionnée",
        ("pr_merged", false, _) => "Une pull request a été fusionnée dans le dépôt",
        ("release_created", true, _) => "Release créée",
        ("release_created", false, _) => "Une Release a été créée dans l’équipe",
        ("generic_event", true, _) => "Événement JSON générique",
        ("generic_event", false, _) => {
            "Un webhook JSON borné et indépendant du fournisseur a été reçu"
        }
        ("daily_at", true, _) => "Tous les jours à une heure locale",
        ("daily_at", false, _) => {
            "Exécuter une fois par jour calendaire local à l’heure configurée"
        }
        ("every_minutes", true, _) => "Toutes les N minutes",
        ("every_minutes", false, _) => "Exécuter selon un intervalle borné en minutes écoulées",
        ("create_incident", true, _) => "Créer un incident",
        ("create_incident", false, _) => {
            "Ouvrir un incident dans l’équipe propriétaire de la règle"
        }
        ("validate_release_step", true, _) => "Valider une étape de Release",
        ("validate_release_step", false, _) => {
            "Valider la prochaine étape séquentielle d’une Release"
        }
        ("block_release", true, _) => "Bloquer une Release",
        ("block_release", false, _) => "Créer et lier un Incident actif à une Release en cours",
        ("escalate_incident", true, _) => "Escalader un Incident",
        ("escalate_incident", false, _) => {
            "Escalader un Incident acquitté en respectant son cycle de vie"
        }
        ("http_notify", true, _) => "Envoyer une notification HTTP",
        ("http_notify", false, _) => "Envoyer une notification via une connexion HTTP configurée",
        ("email_notify", true, _) => "Envoyer un e-mail",
        ("email_notify", false, _) => "Envoyer un e-mail à une adresse configurée",
        _ => fallback,
    }
    .to_string()
}

fn localize_connection(service: &str, locale: &str, fallback: &str) -> String {
    if locale != "fr" {
        return fallback.to_string();
    }
    match service {
        "github" => {
            "Vérifier les webhooks entrants et autoriser facultativement l’accès à l’API GitHub"
        }
        "gitlab" => "Vérifier les webhooks GitLab entrants avec leur jeton secret",
        "generic" => "Recevoir des webhooks JSON bornés authentifiés par un jeton partagé",
        "http" => "Envoyer des notifications bornées vers un endpoint HTTPS public",
        "email" => "Configurer les identifiants SMTP pour l’envoi d’e-mails",
        _ => fallback,
    }
    .to_string()
}

fn localize_oauth(service: &str, locale: &str, fallback: &str, label: bool) -> String {
    if locale == "fr" && service == "github" {
        if label {
            "Autoriser avec GitHub"
        } else {
            "Les jetons d’accès et de rafraîchissement restent chiffrés sur le serveur"
        }
    } else {
        fallback
    }
    .to_string()
}

fn localize_field(name: &str, locale: &str, fallback: &str, label: bool) -> String {
    if locale != "fr" {
        return fallback.to_string();
    }
    if name == "webhook_signing_secret" && fallback == "Webhook secret token" {
        return "Jeton secret du webhook".to_string();
    }
    if name == "webhook_signing_secret"
        && fallback == "Required on first connection; sent by GitLab in X-Gitlab-Token"
    {
        return "Obligatoire à la première connexion ; envoyé par GitLab dans X-Gitlab-Token"
            .to_string();
    }
    if name == "webhook_signing_secret" && fallback == "Shared webhook token" {
        return "Jeton partagé du webhook".to_string();
    }
    if name == "webhook_signing_secret"
        && fallback == "Required on first connection; sent in X-OpsWarden-Token"
    {
        return "Obligatoire à la première connexion ; envoyé dans X-OpsWarden-Token".to_string();
    }
    if name == "severity" && fallback == "Only match this severity" {
        return "Limiter la règle à cette sévérité".to_string();
    }
    match (name, label) {
        ("repository", true) => "Dépôt",
        ("repository", false) => "Limiter la règle à ce dépôt",
        ("workflow", true) => "Workflow",
        ("workflow", false) => "Limiter la règle à ce workflow",
        ("branch", true) => "Branche",
        ("branch", false) => "Limiter la règle à cette branche",
        ("source_branch", true) => "Branche source",
        ("source_branch", false) => "Limiter la règle à cette branche source",
        ("release_id", true) => "Identifiant de Release",
        ("release_id", false) => "UUID de Release ou variable {{release_id}} de l’événement",
        ("release_title", true) => "Titre de Release",
        ("release_title", false) => "Limiter la règle à ce titre exact de Release",
        ("event_type", true) => "Type d’événement",
        ("event_type", false) => "Limiter la règle à la valeur de X-OpsWarden-Event",
        ("source", true) => "Origine",
        ("source", false) => "Limiter la règle à cette source du payload",
        ("external_id", true) => "Identifiant externe",
        ("external_id", false) => "Limiter la règle à cet identifiant externe du payload",
        ("step", true) => "Étape",
        ("step", false) => "Nom exact de la prochaine étape ou template d’événement",
        ("incident_id", true) => "Identifiant d’Incident",
        ("incident_id", false) => {
            "UUID d’un Incident acquitté ou variable {{incident_id}} de l’événement"
        }
        ("tag", true) => "Étiquette Git",
        ("tag", false) => "Limiter la règle à ce tag exact",
        ("conclusion", true) => "Résultat",
        ("conclusion", false) => "Limiter la règle à ce résultat de workflow",
        ("severity", true) => "Sévérité",
        ("severity", false) => "Sévérité affectée à l’incident créé",
        ("title", true) => "Titre de l’incident",
        ("title", false) => {
            "Template facultatif utilisant les variables normalisées de l’événement"
        }
        ("message", true) => "Message",
        ("message", false) => "Template utilisant les variables normalisées de l’événement",
        ("time", true) => "Heure locale",
        ("time", false) => "Heure stricte sur 24 heures au format HH:MM",
        ("timezone", true) => "Fuseau horaire",
        ("timezone", false) => "Fuseau IANA tel que Europe/Paris ou UTC",
        ("minutes", true) => "Intervalle en minutes",
        ("minutes", false) => "Durée entre deux exécutions, de 5 à 1 440 minutes",
        ("webhook_signing_secret", true) => "Secret de signature du webhook",
        ("webhook_signing_secret", false) => {
            "Obligatoire à la première connexion ; laisser vide ensuite pour le conserver"
        }
        ("personal_token", true) => "Jeton d’accès personnel",
        ("personal_token", false) => "Alternative chiffrée facultative à OAuth",
        ("endpoint_url", true) => "URL de l’endpoint",
        ("endpoint_url", false) => {
            "Destination HTTPS publique ; les réseaux locaux et URL avec identifiants sont refusés"
        }
        ("smtp_host", true) => "Hôte SMTP",
        ("smtp_host", false) => "Nom d’hôte ou adresse IP du serveur SMTP",
        ("smtp_port", true) => "Port SMTP",
        ("smtp_port", false) => "Numéro de port (généralement 587 ou 465)",
        ("smtp_username", true) => "Nom d’utilisateur SMTP",
        ("smtp_username", false) => "Nom d’utilisateur pour l’authentification SMTP",
        ("smtp_password", true) => "Mot de passe SMTP",
        ("smtp_password", false) => "Mot de passe pour l’authentification SMTP",
        ("from_address", true) => "Adresse d’expédition",
        ("from_address", false) => "L’adresse e-mail de l’expéditeur",
        ("to", true) => "Destinataire (À)",
        ("to", false) => "L’adresse e-mail de destination",
        ("subject", true) => "Sujet",
        ("subject", false) => "Template utilisant les variables normalisées de l’événement telles que {{repository}}, {{workflow}}, {{tag}} ou {{pull_request_title}}",
        ("body", true) => "Corps du message",
        ("body", false) => "Template utilisant les variables normalisées de l’événement telles que {{repository}}, {{workflow}}, {{tag}} ou {{pull_request_title}}",
        _ => fallback,
    }
    .to_string()
}

fn localize_option(option: &str, locale: &str) -> String {
    if locale != "fr" {
        return match option {
            "low" => "Low",
            "medium" => "Medium",
            "high" => "High",
            "critical" => "Critical",
            _ => option,
        }
        .to_string();
    }
    match option {
        "low" => "Faible",
        "medium" => "Moyenne",
        "high" => "Haute",
        "critical" => "Critique",
        _ => option,
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::{automation_catalog, resolve_client_ip};
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
                for (english_field, french_field) in
                    english_item.fields.iter().zip(&french_item.fields)
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
}
