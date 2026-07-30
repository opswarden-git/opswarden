use axum::{
    extract::{ConnectInfo, Query, State},
    http::{header::HeaderName, HeaderMap},
    Json,
};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
pub mod auth;
pub mod channel;
pub mod error;
pub mod gif;
pub mod incident;
pub mod metrics;
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
        ("alert_firing", true, _) => "Alerte active",
        ("alert_firing", false, _) => "Une alerte Alertmanager est devenue active",
        ("alert_resolved", true, _) => "Alerte résolue",
        ("alert_resolved", false, _) => "Une alerte Alertmanager a été résolue",
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
        "alertmanager" => "Recevoir les groupes Alertmanager authentifiés par un jeton Bearer",
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
    if name == "webhook_signing_secret" && fallback == "Bearer token" {
        return "Jeton Bearer".to_string();
    }
    if name == "webhook_signing_secret" && fallback.contains("Authorization: Bearer") {
        return "Obligatoire à la première connexion ; envoyé dans Authorization: Bearer <jeton>"
            .to_string();
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
        ("alertname", true) => "Nom d’alerte",
        ("alertname", false) => "Limiter la règle au nom partagé par toutes les alertes du groupe",
        ("receiver", true) => "Récepteur",
        ("receiver", false) => "Limiter la règle à ce récepteur Alertmanager",
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
#[path = "mod_tests.rs"]
mod tests;
