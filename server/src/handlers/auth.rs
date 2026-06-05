// server/src/handlers/auth.rs

use axum::{extract::{Extension, State}, Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use crate::AppState;
use crate::app::auth::{SignInCommand, SignInUseCase, SignUpCommand, SignUpUseCase};
use crate::domain::error::DomainError;

#[derive(Deserialize)]
pub struct SignUpPayload {
    pub email: String,
    pub password: String,    
}

#[derive(Serialize)]
pub struct SignUpResponse {
    pub email: String,
    pub status: String,
}

pub async fn sign_up(
    State(state): State<AppState>,
    Json(payload): Json<SignUpPayload>,
) -> Result<(StatusCode, Json<SignUpResponse>), DomainError> {

    let use_case = SignUpUseCase::new(state.users.clone(), state.hasher.clone());

    let command = SignUpCommand {
        email: payload.email,
        plain_password: payload.password,
    };

    // La magie opère ici : si use_case.sign_up retourne une erreur (DomainError),
    // le "?" fait remonter l'erreur, et Axum la transformera automatiquement en HTTP
    // grâce à notre implémentation de IntoResponse dans error.rs !
    let result = use_case.sign_up(command).await?;

    Ok((
        StatusCode::CREATED, 
        Json(SignUpResponse { 
            email: result.email, 
            status: "created".to_string() 
        })
    ))
}

#[derive(Deserialize)]
pub struct SignInPayload {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct SignInResponse {
    pub token: String,
}

pub async fn sign_in(
    State(state): State<AppState>,
    Json(payload): Json<SignInPayload>,
) -> Result<(StatusCode, Json<SignInResponse>), DomainError> {

    let use_case = SignInUseCase::new(state.users.clone(), state.hasher.clone(), state.tokens.clone());

    let command = SignInCommand {
        email: payload.email,
        plain_password: payload.password,
    };

    let result = use_case.sign_in(command).await?;

    Ok((
        StatusCode::OK,
        Json(SignInResponse {
            token: result.token,
        })
    ))
}

#[derive(Serialize)]
pub struct MeResponse {
    pub id: uuid::Uuid,
}

pub async fn get_me(
    Extension(user_id): Extension<uuid::Uuid>,
) -> Result<Json<MeResponse>, DomainError> {
    Ok(Json(MeResponse { id: user_id }))
}