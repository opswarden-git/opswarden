use crate::domain::automation_catalog::{
    ALERTMANAGER_SERVICE, EMAIL_SERVICE, GENERIC_SERVICE, GITHUB_SERVICE, GITLAB_SERVICE,
    HTTP_SERVICE, OPSWARDEN_SERVICE, TIMER_SERVICE,
};
use axum::{
    extract::Query,
    http::{header::HeaderName, HeaderMap},
    Json,
};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
pub mod auth;
mod conversation;
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
    pub server: ServerInfo,
}
#[derive(Serialize)]
pub struct ServerInfo {
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

pub async fn about(Query(query): Query<AboutQuery>) -> Json<About> {
    Json(About {
        server: ServerInfo {
            services: automation_catalog(if query.locale.as_deref() == Some("fr") {
                "fr"
            } else {
                "en"
            }),
        },
    })
}

pub(crate) fn canonical_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(ipv6) => ipv6
            .to_ipv4_mapped()
            .map(IpAddr::V4)
            .unwrap_or(IpAddr::V6(ipv6)),
        ipv4 => ipv4,
    }
}

pub(crate) fn resolve_client_ip(
    peer: IpAddr,
    headers: &HeaderMap,
    trusted_proxy_hops: usize,
) -> IpAddr {
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
                .map(|item| catalog_item(service.service, item, locale))
                .collect(),
            reactions: service
                .reactions
                .iter()
                .map(|item| catalog_item(service.service, item, locale))
                .collect(),
            connection: service.connection.map(|connection| ConnectionCatalog {
                description: localize_connection(service.service, locale, connection.description),
                fields: connection
                    .fields
                    .iter()
                    .map(|field| catalog_field(service.service, "connection", field, locale))
                    .collect(),
                oauth: connection.oauth.map(|oauth| OAuthCatalog {
                    label: localize_oauth(service.service, locale, oauth.label, true),
                    description: localize_oauth(service.service, locale, oauth.description, false),
                }),
                testable: connection.probe.is_some(),
            }),
        })
        .collect()
}

fn catalog_item(
    service: &str,
    item: &crate::domain::automation_catalog::CatalogCapability,
    locale: &str,
) -> CatalogItem {
    CatalogItem {
        name: item.kind.to_string(),
        label: localize_capability(service, item.kind, locale, item.label, true),
        description: localize_capability(service, item.kind, locale, item.description, false),
        connection_service: item.connection_service.map(str::to_string),
        fields: item
            .fields
            .iter()
            .map(|field| catalog_field(service, item.kind, field, locale))
            .collect(),
    }
}

fn catalog_field(
    service: &str,
    owner: &str,
    field: &crate::domain::automation_catalog::CatalogField,
    locale: &str,
) -> CatalogField {
    CatalogField {
        name: field.name.to_string(),
        label: localize_field(service, owner, field.name, locale, field.label, true),
        description: localize_field(service, owner, field.name, locale, field.description, false),
        input_type: field.input_type.to_string(),
        required: field.required,
        default_value: field
            .default_value
            .map(|value| localize_default_value(service, owner, field.name, locale, value)),
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

fn localize_capability(
    service: &str,
    kind: &str,
    locale: &str,
    fallback: &str,
    label: bool,
) -> String {
    if locale != "fr" {
        return fallback.to_string();
    }
    match (service, kind, label) {
        (GITLAB_SERVICE, "ci_failed", true) => "Échec d’une pipeline CI",
        (GITLAB_SERVICE, "ci_failed", false) => {
            "Une pipeline GitLab CI/CD s’est terminée avec un statut en échec"
        }
        (GITHUB_SERVICE, "ci_failed", true) => "Échec d’un workflow CI",
        (GITHUB_SERVICE, "ci_failed", false) => {
            "Un workflow GitHub Actions s’est terminé avec un résultat en échec"
        }
        (GITLAB_SERVICE, "ci_succeeded", true) => "Succès d’une pipeline CI",
        (GITLAB_SERVICE, "ci_succeeded", false) => {
            "Une pipeline GitLab CI/CD s’est terminée avec succès"
        }
        (GITHUB_SERVICE, "ci_succeeded", true) => "Succès d’un workflow CI",
        (GITHUB_SERVICE, "ci_succeeded", false) => {
            "Un workflow GitHub Actions s’est terminé avec succès"
        }
        (GITHUB_SERVICE | GITLAB_SERVICE, "tag_pushed", true) => "Nouveau tag poussé",
        (GITLAB_SERVICE, "tag_pushed", false) => {
            "Un nouveau tag Git a été poussé dans le projet GitLab"
        }
        (GITHUB_SERVICE, "tag_pushed", false) => "Un nouveau tag Git a été poussé dans le dépôt",
        (GITHUB_SERVICE, "pr_merged", true) => "Pull request fusionnée",
        (GITHUB_SERVICE, "pr_merged", false) => "Une pull request a été fusionnée dans le dépôt",
        (OPSWARDEN_SERVICE, "release_created", true) => "Release créée",
        (OPSWARDEN_SERVICE, "release_created", false) => "Une Release a été créée dans l’équipe",
        (GENERIC_SERVICE, "generic_event", true) => "Événement JSON générique",
        (GENERIC_SERVICE, "generic_event", false) => {
            "Un webhook JSON borné et indépendant du fournisseur a été reçu"
        }
        (ALERTMANAGER_SERVICE, "alert_firing", true) => "Alerte active",
        (ALERTMANAGER_SERVICE, "alert_firing", false) => {
            "Une alerte Alertmanager est devenue active"
        }
        (ALERTMANAGER_SERVICE, "alert_resolved", true) => "Alerte résolue",
        (ALERTMANAGER_SERVICE, "alert_resolved", false) => "Une alerte Alertmanager a été résolue",
        (TIMER_SERVICE, "daily_at", true) => "Tous les jours à une heure locale",
        (TIMER_SERVICE, "daily_at", false) => {
            "Exécuter une fois par jour calendaire local à l’heure configurée"
        }
        (TIMER_SERVICE, "every_minutes", true) => "Toutes les N minutes",
        (TIMER_SERVICE, "every_minutes", false) => {
            "Exécuter selon un intervalle borné en minutes écoulées"
        }
        (OPSWARDEN_SERVICE, "create_incident", true) => "Créer un incident",
        (OPSWARDEN_SERVICE, "create_incident", false) => {
            "Ouvrir un incident dans l’équipe propriétaire de la règle"
        }
        (OPSWARDEN_SERVICE, "validate_release_step", true) => "Valider une étape de Release",
        (OPSWARDEN_SERVICE, "validate_release_step", false) => {
            "Valider la prochaine étape séquentielle d’une Release"
        }
        (OPSWARDEN_SERVICE, "block_release", true) => "Bloquer une Release",
        (OPSWARDEN_SERVICE, "block_release", false) => {
            "Créer et lier un Incident actif à une Release en cours"
        }
        (OPSWARDEN_SERVICE, "escalate_incident", true) => "Escalader un Incident",
        (OPSWARDEN_SERVICE, "escalate_incident", false) => {
            "Escalader un Incident acquitté en respectant son cycle de vie"
        }
        (HTTP_SERVICE, "http_notify", true) => "Envoyer une notification HTTP",
        (HTTP_SERVICE, "http_notify", false) => {
            "Envoyer une notification via une connexion HTTP configurée"
        }
        (EMAIL_SERVICE, "email_notify", true) => "Envoyer un e-mail",
        (EMAIL_SERVICE, "email_notify", false) => "Envoyer un e-mail à une adresse configurée",
        _ => fallback,
    }
    .to_string()
}

fn localize_connection(service: &str, locale: &str, fallback: &str) -> String {
    if locale != "fr" {
        return fallback.to_string();
    }
    match service {
        GITHUB_SERVICE => {
            "Vérifier les webhooks entrants et autoriser facultativement l’accès à l’API GitHub"
        }
        GITLAB_SERVICE => "Vérifier les webhooks GitLab entrants avec leur jeton secret",
        GENERIC_SERVICE => "Recevoir des webhooks JSON bornés authentifiés par un jeton partagé",
        ALERTMANAGER_SERVICE => {
            "Recevoir les groupes Alertmanager authentifiés par un jeton Bearer"
        }
        HTTP_SERVICE => "Envoyer des notifications bornées vers un endpoint HTTPS public",
        EMAIL_SERVICE => "Configurer les identifiants SMTP pour l’envoi d’e-mails",
        _ => fallback,
    }
    .to_string()
}

fn localize_oauth(service: &str, locale: &str, fallback: &str, label: bool) -> String {
    if locale == "fr" && service == GITHUB_SERVICE {
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

fn localize_field(
    service: &str,
    owner: &str,
    name: &str,
    locale: &str,
    fallback: &str,
    label: bool,
) -> String {
    if locale != "fr" {
        return fallback.to_string();
    }
    let qualified = match (service, owner, name, label) {
        (GITHUB_SERVICE, "pr_merged", "branch", true) => "Branche cible",
        (GITHUB_SERVICE, "pr_merged", "branch", false) => "Limiter la règle à cette branche cible",
        (OPSWARDEN_SERVICE, "release_created", "release_id", false) => {
            "Limiter la règle à cette Release"
        }
        (GENERIC_SERVICE, "generic_event", "severity", false) => {
            "Limiter la règle à cette sévérité"
        }
        (ALERTMANAGER_SERVICE, "alert_firing" | "alert_resolved", "severity", false) => {
            "Limiter la règle à cette sévérité d’alerte"
        }
        (TIMER_SERVICE, "every_minutes", "timezone", false) => {
            "Fuseau IANA utilisé pour afficher le contexte d’exécution"
        }
        (OPSWARDEN_SERVICE, "block_release", "severity", true) => "Sévérité du blocage",
        (OPSWARDEN_SERVICE, "block_release", "severity", false) => {
            "Sévérité affectée à l’Incident bloquant"
        }
        (OPSWARDEN_SERVICE, "block_release", "title", true) => "Titre du blocage",
        (OPSWARDEN_SERVICE, "block_release", "title", false) => {
            "Template facultatif du titre de l’Incident"
        }
        (GITHUB_SERVICE, "connection", "webhook_signing_secret", true) => {
            "Secret de signature du webhook"
        }
        (GITHUB_SERVICE, "connection", "webhook_signing_secret", false) => {
            "Obligatoire à la première connexion ; laisser vide ensuite pour le conserver"
        }
        (GITLAB_SERVICE, "connection", "webhook_signing_secret", true) => "Jeton secret du webhook",
        (GITLAB_SERVICE, "connection", "webhook_signing_secret", false) => {
            "Obligatoire à la première connexion ; envoyé par GitLab dans X-Gitlab-Token"
        }
        (GENERIC_SERVICE, "connection", "webhook_signing_secret", true) => {
            "Jeton partagé du webhook"
        }
        (GENERIC_SERVICE, "connection", "webhook_signing_secret", false) => {
            "Obligatoire à la première connexion ; envoyé dans X-OpsWarden-Token"
        }
        (ALERTMANAGER_SERVICE, "connection", "webhook_signing_secret", true) => "Jeton Bearer",
        (ALERTMANAGER_SERVICE, "connection", "webhook_signing_secret", false) => {
            "Obligatoire à la première connexion ; envoyé dans Authorization: Bearer <jeton>"
        }
        _ => "",
    };
    if !qualified.is_empty() {
        return qualified.to_string();
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
        ("title", false) => "Template facultatif utilisant les variables normalisées de l’événement",
        ("message", true) => "Message",
        ("message", false) => "Template utilisant les variables normalisées de l’événement",
        ("time", true) => "Heure locale",
        ("time", false) => "Heure stricte sur 24 heures au format HH:MM",
        ("timezone", true) => "Fuseau horaire",
        ("timezone", false) => "Fuseau IANA tel que Europe/Paris ou UTC",
        ("minutes", true) => "Intervalle en minutes",
        ("minutes", false) => "Durée entre deux exécutions, de 5 à 1 440 minutes",
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

fn localize_default_value(
    service: &str,
    owner: &str,
    name: &str,
    locale: &str,
    fallback: &str,
) -> String {
    if locale == "fr" {
        match (service, owner, name) {
            (HTTP_SERVICE, "http_notify", "message")
            | (EMAIL_SERVICE, "email_notify", "subject" | "body") => {
                return "Événement d’automatisation sur {{repository}}".to_string();
            }
            _ => {}
        }
    }
    fallback.to_string()
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
