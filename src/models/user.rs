use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::{Validate, ValidationError};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub bio: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUser {
    pub username: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, max = 64), custom(function = "validate_password"))]
    pub password: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub bio: Option<String>,
}
fn validate_password(password: &str) -> Result<(), ValidationError> {
    let mut errors = Vec::new();

    if !password.chars().any(|c| c.is_uppercase()) {
        errors.push("uppercase letter required");
    }
    if !password.chars().any(|c| c.is_lowercase()) {
        errors.push("lowercase letter required");
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        errors.push("digit required");
    }
    if !password
        .chars()
        .any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c))
    {
        errors.push("special character required");
    }

    if !errors.is_empty() {
        let error_message = errors.join(", ");
        let mut error = ValidationError::new("password_requirements");
        error.message = Some(format!("Password requirements not met: {error_message}").into());
        return Err(error);
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct UpdateUser {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub bio: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UserSearch {
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl Default for UserSearch {
    fn default() -> Self {
        Self {
            search: None,
            limit: Some(20),
            offset: Some(0),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UserListResponse {
    pub users: Vec<User>,
    pub total_count: i64,
    pub limit: i64,
    pub offset: i64,
}
