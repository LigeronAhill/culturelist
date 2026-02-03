use anyhow::Result;
use sqlx::{Pool, Postgres};

use crate::models::{CreateUser, UpdateUser, User, UserListResponse, UserSearch};

pub struct UsersStorage {
    pool: Pool<Postgres>,
}

impl UsersStorage {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
    pub async fn create(&self, data: CreateUser) -> Result<User> {
        let password_hash = hash_password(&data.password)?;
        let result = sqlx::query_file_as!(
            User,
            "queries/users/create.sql",
            data.username,
            data.email,
            password_hash,
            data.first_name,
            data.last_name,
            data.bio,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(result)
    }
    pub async fn verify_user(&self, email: &str, password: &str) -> Result<bool> {
        let password_hash: Option<String> =
            sqlx::query_scalar("SELECT password FROM users WHERE email = $1")
                .bind(email)
                .fetch_optional(&self.pool)
                .await?;
        let res = password_hash
            .and_then(|hash| verify_password(&hash, password).ok())
            .ok_or(anyhow::anyhow!("wrong credentials"))?;
        Ok(res)
    }
    pub async fn get_by_email(&self, email: &str) -> Result<Option<User>> {
        let res = sqlx::query_file_as!(User, "queries/users/get_by_email.sql", email,)
            .fetch_optional(&self.pool)
            .await?;
        Ok(res)
    }
    pub async fn get_by_id(&self, id: uuid::Uuid) -> Result<Option<User>> {
        let res = sqlx::query_file_as!(User, "queries/users/get_by_id.sql", id,)
            .fetch_optional(&self.pool)
            .await?;
        Ok(res)
    }
    pub async fn list_users(&self, data: UserSearch) -> Result<UserListResponse> {
        let total_count = sqlx::query_file_scalar!("queries/users/list_count.sql", data.search)
            .fetch_one(&self.pool)
            .await?
            .unwrap_or_default();
        // Empty results are valid, continue with empty user list
        let limit = data.limit.unwrap_or(20);
        let offset = data.offset.unwrap_or(0);

        let users =
            sqlx::query_file_as!(User, "queries/users/list.sql", data.search, limit, offset,)
                .fetch_all(&self.pool)
                .await?;

        let result = UserListResponse {
            users,
            total_count,
            limit,
            offset,
        };
        Ok(result)
    }
    pub async fn update(&self, id: uuid::Uuid, data: UpdateUser) -> Result<Option<User>> {
        let result = sqlx::query_file_as!(
            User,
            "queries/users/update.sql",
            id,
            data.username,
            data.email,
            data.password,
            data.first_name,
            data.last_name,
            data.bio,
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(result)
    }
    pub async fn delete(&self, id: uuid::Uuid) -> Result<Option<uuid::Uuid>> {
        let result = sqlx::query_file_scalar!("queries/users/delete.sql", id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(result)
    }
}

fn hash_password(password: &str) -> Result<String> {
    use argon2::{
        Argon2,
        password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
    };
    let salt = SaltString::generate(&mut OsRng);

    // Argon2 with default params (Argon2id v19)
    let argon2 = Argon2::default();

    // Hash password to PHC string ($argon2id$v=19$...)
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?
        .to_string();
    Ok(password_hash)
}

fn verify_password(password_hash: &str, password: &str) -> Result<bool> {
    use argon2::{
        Argon2,
        password_hash::{PasswordHash, PasswordVerifier},
    };
    let parsed_hash =
        PasswordHash::new(password_hash).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let res = Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok();
    Ok(res)
}
