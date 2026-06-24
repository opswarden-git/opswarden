// --- server/src/handlers/gif.rs ---
//
// Authenticated GIF search proxy: GET /api/giphy/search?q=&limit=&rating=. The
// GIPHY key never leaves the server; the handler validates via the use-case and
// returns normalized results.

use axum::{
    extract::{Extension, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::app::gif::{SearchGifsCommand, SearchGifsUseCase};
use crate::domain::error::DomainError;
use crate::handlers::middleware::AuthenticatedSession;
use crate::AppState;

#[derive(Deserialize)]
pub struct GifSearchQuery {
    pub q: String,
    pub limit: Option<u32>,
    pub rating: Option<String>,
}

#[derive(Serialize)]
pub struct GifResponse {
    pub id: String,
    pub title: String,
    pub url: String,
    pub preview_url: String,
    pub width: u32,
    pub height: u32,
}

pub async fn search_gifs(
    State(state): State<AppState>,
    // Extraction enforces the logged-in gate (this route lives under the auth
    // middleware); the search itself does not need the user id.
    Extension(_session): Extension<AuthenticatedSession>,
    Query(query): Query<GifSearchQuery>,
) -> Result<Json<Vec<GifResponse>>, DomainError> {
    let use_case = SearchGifsUseCase::new(state.gifs.clone());
    let results = use_case
        .search(SearchGifsCommand {
            query: query.q,
            limit: query.limit,
            rating: query.rating,
        })
        .await?;

    Ok(Json(
        results
            .into_iter()
            .map(|g| GifResponse {
                id: g.id,
                title: g.title,
                url: g.url,
                preview_url: g.preview_url,
                width: g.width,
                height: g.height,
            })
            .collect(),
    ))
}
