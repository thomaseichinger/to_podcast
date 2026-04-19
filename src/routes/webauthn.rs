use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use webauthn_rs::prelude::*;

use crate::app::AppState;
use crate::storage;

#[derive(Deserialize)]
pub struct UsernameReq {
    pub username: String,
}

#[derive(Deserialize)]
pub struct RegisterFinishReq {
    pub username: String,
    pub credential: RegisterPublicKeyCredential,
}

#[derive(Deserialize)]
pub struct AuthFinishReq {
    pub username: String,
    pub credential: PublicKeyCredential,
}

#[derive(Serialize)]
pub struct TokenResp {
    pub token: String,
}

// --- Registration ---

pub async fn register_start(
    State(state): State<AppState>,
    Json(req): Json<UsernameReq>,
) -> impl IntoResponse {
    let username = req.username.trim().to_string();
    if username.is_empty() {
        return (StatusCode::BAD_REQUEST, "username required").into_response();
    }

    let user_id = match storage::get_or_create_user(&state.pool, &username).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("get_or_create_user: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    match state
        .webauthn
        .start_passkey_registration(user_id, &username, &username, None)
    {
        Ok((ccr, reg_state)) => {
            match serde_json::to_string(&reg_state) {
                Ok(json) => {
                    state.reg_states.lock().unwrap().insert(username, json);
                    Json(ccr).into_response()
                }
                Err(e) => {
                    tracing::error!("serialize reg_state: {e}");
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
        Err(e) => {
            tracing::error!("start_passkey_registration: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

pub async fn register_finish(
    State(state): State<AppState>,
    Json(req): Json<RegisterFinishReq>,
) -> impl IntoResponse {
    let reg_json = match state.reg_states.lock().unwrap().remove(&req.username) {
        Some(j) => j,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                "No pending registration for that username",
            )
                .into_response()
        }
    };

    let reg_state: PasskeyRegistration = match serde_json::from_str(&reg_json) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("deserialize reg_state: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    match state
        .webauthn
        .finish_passkey_registration(&req.credential, &reg_state)
    {
        Ok(passkey) => {
            let cred_json = match serde_json::to_string(&passkey) {
                Ok(j) => j,
                Err(e) => {
                    tracing::error!("serialize passkey: {e}");
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            };
            let user_id = match storage::get_or_create_user(&state.pool, &req.username).await {
                Ok(id) => id,
                Err(e) => {
                    tracing::error!("get_or_create_user: {e}");
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            };
            if let Err(e) =
                storage::save_credential(&state.pool, user_id, &req.username, &cred_json).await
            {
                tracing::error!("save_credential: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
            StatusCode::OK.into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

// --- Authentication ---

pub async fn auth_start(
    State(state): State<AppState>,
    Json(req): Json<UsernameReq>,
) -> impl IntoResponse {
    let username = req.username.trim().to_string();

    let cred_jsons = match storage::get_credentials_for_user(&state.pool, &username).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("get_credentials_for_user: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if cred_jsons.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            "No credentials registered for that username",
        )
            .into_response();
    }

    let passkeys: Vec<Passkey> = cred_jsons
        .iter()
        .filter_map(|j| serde_json::from_str(j).ok())
        .collect();

    if passkeys.is_empty() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not parse stored credentials",
        )
            .into_response();
    }

    match state.webauthn.start_passkey_authentication(&passkeys) {
        Ok((rcr, auth_state)) => match serde_json::to_string(&auth_state) {
            Ok(json) => {
                state.auth_states.lock().unwrap().insert(username, json);
                Json(rcr).into_response()
            }
            Err(e) => {
                tracing::error!("serialize auth_state: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
        Err(e) => {
            tracing::error!("start_passkey_authentication: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

pub async fn auth_finish(
    State(state): State<AppState>,
    Json(req): Json<AuthFinishReq>,
) -> impl IntoResponse {
    let auth_json = match state.auth_states.lock().unwrap().remove(&req.username) {
        Some(j) => j,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                "No pending authentication for that username",
            )
                .into_response()
        }
    };

    let auth_state: PasskeyAuthentication = match serde_json::from_str(&auth_json) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("deserialize auth_state: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    match state
        .webauthn
        .finish_passkey_authentication(&req.credential, &auth_state)
    {
        Ok(_) => {
            let token = state.sessions.create_session();
            Json(TokenResp { token }).into_response()
        }
        Err(e) => (StatusCode::UNAUTHORIZED, e.to_string()).into_response(),
    }
}
