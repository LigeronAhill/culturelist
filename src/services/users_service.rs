use std::{error::Error, fmt::Display};

use axum::{http::StatusCode, response::IntoResponse};
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationErrors};

use crate::{
    models::{
        CreateUser, SignInRequest, SignInResponse, SignUpRequest, SignUpResponse, UpdateUser, User,
        UserListResponse, UserSearch,
    },
    storage::UsersStorage,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UsersServiceError {
    NotFound,
    WrongCredentials(String),
    DatabaseError(String),
    VerificationError(String),
}
impl From<sqlx::Error> for UsersServiceError {
    fn from(value: sqlx::Error) -> Self {
        Self::DatabaseError(value.to_string())
    }
}
impl Display for UsersServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl IntoResponse for UsersServiceError {
    fn into_response(self) -> axum::response::Response {
        match self {
            UsersServiceError::NotFound => StatusCode::NOT_FOUND.into_response(),
            UsersServiceError::WrongCredentials(err) => {
                (StatusCode::BAD_REQUEST, err).into_response()
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
impl Error for UsersServiceError {}
impl From<ValidationErrors> for UsersServiceError {
    fn from(value: ValidationErrors) -> Self {
        let mut res = Vec::new();
        for (field, err) in value.errors() {
            match err {
                validator::ValidationErrorsKind::Field(validation_errors) => {
                    for error in validation_errors {
                        if let Some(message) = error.message.as_ref() {
                            let m = format!("{field}: {message}");
                            res.push(m);
                        }
                    }
                }
                _ => res.push("Wrong credentials".into()),
            }
        }
        let s = res.join(";");
        Self::WrongCredentials(s)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user id
    pub email: String,
    pub exp: usize, // expiration time
}

#[derive(Clone, Debug)]
pub struct UsersService {
    storage: UsersStorage,
}

impl UsersService {
    pub fn new(storage: UsersStorage) -> Self {
        Self { storage }
    }

    fn generate_jwt_token(&self, user: &User) -> Result<String, UsersServiceError> {
        let expiration = Utc::now()
            .checked_add_signed(Duration::days(7))
            .expect("valid timestamp")
            .timestamp() as usize;

        let claims = Claims {
            sub: user.id.to_string(),
            email: user.email.clone(),
            exp: expiration,
        };

        let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "your-secret-key".to_string());
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .map_err(|e| {
            UsersServiceError::DatabaseError(format!("Failed to generate token: {}", e))
        })?;

        Ok(token)
    }

    pub async fn sign_in(
        &self,
        credentials: SignInRequest,
    ) -> Result<SignInResponse, UsersServiceError> {
        credentials.validate()?;

        let user = self
            .storage
            .get_by_email(&credentials.email)
            .await
            .map_err(|e| UsersServiceError::DatabaseError(e.to_string()))?
            .ok_or(UsersServiceError::WrongCredentials(
                "Invalid email or password".to_string(),
            ))?;

        let is_valid = self
            .storage
            .verify_user(&credentials.email, &credentials.password)
            .await
            .map_err(|e| UsersServiceError::VerificationError(e.to_string()))?;

        if !is_valid {
            return Err(UsersServiceError::WrongCredentials(
                "Invalid email or password".to_string(),
            ));
        }

        let token = self.generate_jwt_token(&user)?;
        Ok(SignInResponse { user, token })
    }

    pub async fn sign_up(
        &self,
        user_data: SignUpRequest,
    ) -> Result<SignUpResponse, UsersServiceError> {
        user_data.validate()?;

        // Check if user already exists
        if let Ok(Some(_)) = self.storage.get_by_email(&user_data.email).await {
            return Err(UsersServiceError::WrongCredentials(
                "Email already exists".to_string(),
            ));
        }

        let create_user = CreateUser {
            username: user_data.username,
            email: user_data.email,
            password: user_data.password,
            first_name: user_data.first_name,
            last_name: user_data.last_name,
            bio: user_data.bio,
        };

        let user = self
            .storage
            .create(create_user)
            .await
            .map_err(|e| UsersServiceError::DatabaseError(e.to_string()))?;

        let token = self.generate_jwt_token(&user)?;
        Ok(SignUpResponse { user, token })
    }

    pub async fn create(&self, data: CreateUser) -> Result<User, UsersServiceError> {
        data.validate()?;
        let created = self
            .storage
            .create(data)
            .await
            .map_err(|e| UsersServiceError::DatabaseError(e.to_string()))?;
        Ok(created)
    }
    pub async fn get_by_email(&self, email: &str) -> Result<User, UsersServiceError> {
        let existing = self
            .storage
            .get_by_email(email)
            .await
            .map_err(|e| UsersServiceError::DatabaseError(e.to_string()))?
            .ok_or(UsersServiceError::NotFound)?;
        Ok(existing)
    }
    pub async fn get_by_id(&self, id: &str) -> Result<User, UsersServiceError> {
        let parsed = uuid::Uuid::parse_str(id)
            .map_err(|_| UsersServiceError::WrongCredentials("Wrong id format".into()))?;
        let existing = self
            .storage
            .get_by_id(parsed)
            .await
            .map_err(|e| UsersServiceError::DatabaseError(e.to_string()))?
            .ok_or(UsersServiceError::NotFound)?;
        Ok(existing)
    }
    pub async fn list(
        &self,
        page: u32,
        per_page: u32,
        search_query: Option<String>,
    ) -> Result<UserListResponse, UsersServiceError> {
        if page == 0 {
            return Err(UsersServiceError::WrongCredentials(
                "Page must be greater than zero".into(),
            ));
        }
        let filter = UserSearch {
            search: search_query,
            limit: Some(per_page as i64),
            offset: Some(((page - 1) * per_page) as i64),
        };
        let result = self
            .storage
            .list_users(filter)
            .await
            .map_err(|e| UsersServiceError::DatabaseError(e.to_string()))?;
        if result.users.is_empty() {
            return Err(UsersServiceError::NotFound);
        }
        Ok(result)
    }
    pub async fn update(
        &self,
        user_id: &str,
        data: UpdateUser,
        old_password: Option<String>,
    ) -> Result<User, UsersServiceError> {
        let existing_user = self.get_by_id(user_id).await?;
        if data.password.as_ref().is_some() {
            match old_password {
                Some(old) => {
                    let verified = self
                        .storage
                        .verify_user(&existing_user.email, &old)
                        .await
                        .map_err(|e| UsersServiceError::VerificationError(e.to_string()))?;
                    if !verified {
                        return Err(UsersServiceError::WrongCredentials(
                            "Wrong old password".into(),
                        ));
                    }
                }
                None => {
                    return Err(UsersServiceError::WrongCredentials(
                        "To change password please provide old password".into(),
                    ));
                }
            }
        }
        match self
            .storage
            .update(existing_user.id, data)
            .await
            .map_err(|e| UsersServiceError::DatabaseError(e.to_string()))?
        {
            Some(u) => Ok(u),
            None => Err(UsersServiceError::NotFound),
        }
    }
    pub async fn delete(&self, id: &str) -> Result<uuid::Uuid, UsersServiceError> {
        let parsed = uuid::Uuid::parse_str(id)
            .map_err(|_| UsersServiceError::WrongCredentials("Wrong id format".into()))?;
        let deleted_id = self
            .storage
            .delete(parsed)
            .await
            .map_err(|e| UsersServiceError::DatabaseError(e.to_string()))?
            .ok_or(UsersServiceError::NotFound)?;
        Ok(deleted_id)
    }
    pub async fn check_username_exists(&self, username: &str) -> Result<bool, UsersServiceError> {
        let existing = self.storage.get_by_username(username).await?;
        Ok(existing.is_some())
    }
}
