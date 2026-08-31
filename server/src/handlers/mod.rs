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

include!("catalog_i18n.rs");

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
