use std::{error::Error, fmt::Display};

use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationErrors};

use crate::{
    models::{CreateUser, UpdateUser, User, UserListResponse, UserSearch},
    storage::UsersStorage,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UsersServiceError {
    NotFound,
    WrongCredentials(String),
    DatabaseError(String),
    VerificationError(String),
}
impl Display for UsersServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
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

pub struct UsersService {
    storage: UsersStorage,
}

impl UsersService {
    pub fn new(storage: UsersStorage) -> Self {
        Self { storage }
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
}
