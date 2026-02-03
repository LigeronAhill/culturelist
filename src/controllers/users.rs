use serde::{Deserialize, Serialize};
use std::sync::Arc;

use axum::{
    Json, debug_handler,
    extract::{Path, State},
};

use crate::{
    AppState,
    models::{
        CreateUser, SignInRequest, SignInResponse, SignUpRequest, SignUpResponse, UpdateUser, User,
        UserListResponse,
    },
    services::UsersServiceError,
};

#[debug_handler]
pub async fn sign_in(
    State(state): State<Arc<AppState>>,
    Json(credentials): Json<SignInRequest>,
) -> Result<Json<SignInResponse>, UsersServiceError> {
    let response = state.users_service.sign_in(credentials).await?;
    Ok(Json(response))
}

#[debug_handler]
pub async fn sign_up(
    State(state): State<Arc<AppState>>,
    Json(user_data): Json<SignUpRequest>,
) -> Result<Json<SignUpResponse>, UsersServiceError> {
    let response = state.users_service.sign_up(user_data).await?;
    Ok(Json(response))
}

#[debug_handler]
pub async fn create_user(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateUser>,
) -> Result<Json<User>, UsersServiceError> {
    let created = state.users_service.create(payload).await?;
    Ok(Json(created))
}
pub async fn get_user_by_id(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<User>, UsersServiceError> {
    let user = state.users_service.get_by_id(&id).await?;
    Ok(Json(user))
}

#[derive(Deserialize)]
pub struct ListUsersRequest {
    pub page: u32,
    pub per_page: u32,
    pub search_query: Option<String>,
}

pub async fn list_users(
    State(state): State<Arc<AppState>>,
    Json(data): Json<ListUsersRequest>,
) -> Result<Json<UserListResponse>, UsersServiceError> {
    let result = state
        .users_service
        .list(data.page, data.per_page, data.search_query)
        .await?;
    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub old_password: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub bio: Option<String>,
}

pub async fn update_user(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(data): Json<UpdateUserRequest>,
) -> Result<Json<User>, UsersServiceError> {
    let upd = UpdateUser {
        username: data.username,
        email: data.email,
        password: data.password,
        first_name: data.first_name,
        last_name: data.last_name,
        bio: data.bio,
    };
    let updated = state
        .users_service
        .update(&id, upd, data.old_password)
        .await?;
    Ok(Json(updated))
}

#[derive(Debug, Serialize)]
pub struct DeleteUserResponse {
    pub deleted_id: uuid::Uuid,
}

pub async fn delete_user(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<DeleteUserResponse>, UsersServiceError> {
    let deleted_id = state.users_service.delete(&id).await?;
    Ok(Json(DeleteUserResponse { deleted_id }))
}
