use anyhow::Result;
use axum_session_auth::Authentication;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::{Validate, ValidationError};

use crate::services::UsersService;

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

impl Default for User {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            username: String::new(),
            email: String::new(),
            first_name: None,
            last_name: None,
            bio: None,
            created_at: Utc::now(),
        }
    }
}

#[async_trait::async_trait]
impl Authentication<User, String, UsersService> for User {
    async fn load_user(userid: String, service: Option<&UsersService>) -> Result<User> {
        let user = service.unwrap().get_by_id(&userid).await?;
        Ok(user)
    }

    fn is_authenticated(&self) -> bool {
        self.id != Uuid::nil()
    }

    fn is_active(&self) -> bool {
        self.id != Uuid::nil()
    }

    fn is_anonymous(&self) -> bool {
        self.id == Uuid::nil()
    }
}

#[derive(Debug, Clone, Deserialize, Validate)]
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

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Deserialize, Validate)]
pub struct SignInRequest {
    #[validate(email)]
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct SignInResponse {
    pub user: User,
    pub token: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SignUpRequest {
    pub username: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, max = 64), custom(function = "validate_password"))]
    pub password: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub bio: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SignUpResponse {
    pub user: User,
    pub token: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_validation_success() {
        // Valid password with all requirements
        let valid_password = "Password123!";
        assert!(validate_password(valid_password).is_ok());
    }

    #[test]
    fn test_password_validation_missing_uppercase() {
        let invalid_password = "password123!";
        let result = validate_password(invalid_password);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.to_string().contains("uppercase letter required"));
    }

    #[test]
    fn test_password_validation_missing_lowercase() {
        let invalid_password = "PASSWORD123!";
        let result = validate_password(invalid_password);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.to_string().contains("lowercase letter required"));
    }

    #[test]
    fn test_password_validation_missing_digit() {
        let invalid_password = "Password!";
        let result = validate_password(invalid_password);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.to_string().contains("digit required"));
    }

    #[test]
    fn test_password_validation_missing_special() {
        let invalid_password = "Password123";
        let result = validate_password(invalid_password);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.to_string().contains("special character required"));
    }

    #[test]
    fn test_password_validation_multiple_errors() {
        let invalid_password = "weak";
        let result = validate_password(invalid_password);
        assert!(result.is_err());

        let error = result.unwrap_err();
        let error_string = error.to_string();

        // Should contain multiple error messages
        assert!(error_string.contains("uppercase letter required"));
        assert!(error_string.contains("digit required"));
        assert!(error_string.contains("special character required"));
    }

    #[test]
    fn test_create_user_validation_success() {
        let valid_user = CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "Password123!".to_string(),
            first_name: Some("Test".to_string()),
            last_name: Some("User".to_string()),
            bio: Some("Test user bio".to_string()),
        };

        assert!(valid_user.validate().is_ok());
    }

    #[test]
    fn test_create_user_validation_invalid_email() {
        let invalid_user = CreateUser {
            username: "testuser".to_string(),
            email: "invalid-email".to_string(),
            password: "Password123!".to_string(),
            first_name: None,
            last_name: None,
            bio: None,
        };

        let result = invalid_user.validate();
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("email"));
    }

    #[test]
    fn test_create_user_validation_password_too_short() {
        let invalid_user = CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "short".to_string(),
            first_name: None,
            last_name: None,
            bio: None,
        };

        let result = invalid_user.validate();
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("password"));
    }

    #[test]
    fn test_create_user_validation_password_too_long() {
        let long_password = "a".repeat(65); // 65 characters, exceeds max 64
        let invalid_user = CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: long_password,
            first_name: None,
            last_name: None,
            bio: None,
        };

        let result = invalid_user.validate();
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("password"));
    }

    #[test]
    fn test_create_user_validation_password_complexity() {
        let invalid_user = CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "weakpassword".to_string(), // Missing digit and special
            first_name: None,
            last_name: None,
            bio: None,
        };

        let result = invalid_user.validate();
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("password"));
    }

    #[test]
    fn test_sign_in_request_validation_success() {
        let valid_signin = SignInRequest {
            email: "test@example.com".to_string(),
            password: "anypassword".to_string(),
        };

        assert!(valid_signin.validate().is_ok());
    }

    #[test]
    fn test_sign_in_request_validation_invalid_email() {
        let invalid_signin = SignInRequest {
            email: "invalid-email".to_string(),
            password: "anypassword".to_string(),
        };

        let result = invalid_signin.validate();
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("email"));
    }

    #[test]
    fn test_sign_in_request_validation_empty_email() {
        let invalid_signin = SignInRequest {
            email: "".to_string(),
            password: "anypassword".to_string(),
        };

        let result = invalid_signin.validate();
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("email"));
    }

    #[test]
    fn test_sign_in_request_validation_empty_password() {
        let invalid_signin = SignInRequest {
            email: "test@example.com".to_string(),
            password: "".to_string(),
        };

        // SignInRequest only validates email field, password can be empty for validation
        // (actual password verification happens in service layer)
        assert!(invalid_signin.validate().is_ok());
    }

    #[test]
    fn test_sign_up_request_validation_success() {
        let valid_signup = SignUpRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "Password123!".to_string(),
            first_name: Some("Test".to_string()),
            last_name: Some("User".to_string()),
            bio: Some("Test user bio".to_string()),
        };

        assert!(valid_signup.validate().is_ok());
    }

    #[test]
    fn test_sign_up_request_validation_all_fields_invalid() {
        let invalid_signup = SignUpRequest {
            username: "testuser".to_string(),
            email: "invalid-email".to_string(),
            password: "weak".to_string(),
            first_name: None,
            last_name: None,
            bio: None,
        };

        let result = invalid_signup.validate();
        assert!(result.is_err());

        let errors = result.unwrap_err();
        // Should have validation errors for both email and password
        assert!(errors.field_errors().contains_key("email"));
        assert!(errors.field_errors().contains_key("password"));
    }

    #[test]
    fn test_user_search_default_values() {
        let default_search = UserSearch::default();
        assert_eq!(default_search.search, None);
        assert_eq!(default_search.limit, Some(20));
        assert_eq!(default_search.offset, Some(0));
    }

    #[test]
    fn test_user_search_custom_values() {
        let custom_search = UserSearch {
            search: Some("query".to_string()),
            limit: Some(10),
            offset: Some(50),
        };

        assert_eq!(custom_search.search, Some("query".to_string()));
        assert_eq!(custom_search.limit, Some(10));
        assert_eq!(custom_search.offset, Some(50));
    }

    #[test]
    fn test_password_with_edge_cases() {
        // Test password with special characters at boundaries
        let edge_case_passwords = vec![
            "A1!aaaaa",       // Minimum valid length
            "Password123@#%", // Multiple special chars
            "12345A!a",       // Special chars mixed with numbers
            "!@#Aa1b2c3",     // Special chars at start
        ];

        for password in edge_case_passwords {
            assert!(
                validate_password(password).is_ok(),
                "Password '{}' should be valid",
                password
            );
        }
    }

    #[test]
    fn test_password_special_characters() {
        // Test all supported special characters
        let special_chars = "!@#$%^&*()_+-=[]{}|;:,.<>?";
        for special_char in special_chars.chars() {
            let password = format!("Password123{}", special_char);
            assert!(
                validate_password(&password).is_ok(),
                "Password with special character '{}' should be valid",
                special_char
            );
        }
    }
}
