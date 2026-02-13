use sqlx::{Pool, Postgres, Result};

use crate::models::{CreateUser, UpdateUser, User, UserListResponse, UserSearch};

#[derive(Clone, Debug)]
pub struct UsersStorage {
    pool: Pool<Postgres>,
}

impl UsersStorage {
    pub async fn new(pool: Pool<Postgres>) -> Result<Self> {
        let storage = Self { pool };
        Ok(storage)
    }
    pub async fn create(&self, data: CreateUser) -> Result<User> {
        let password_hash =
            hash_password(&data.password).map_err(|_| sqlx::Error::WorkerCrashed)?;
        let result = sqlx::query_file_as!(
            User,
            "queries/users/create.sql",
            data.username,
            data.email.to_lowercase(),
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
                .bind(email.to_lowercase())
                .fetch_optional(&self.pool)
                .await?;
        let res = password_hash
            .and_then(|hash| verify_password(&hash, password).ok())
            .ok_or(sqlx::Error::WorkerCrashed)?;
        Ok(res)
    }
    pub async fn get_by_email(&self, email: &str) -> Result<Option<User>> {
        let res =
            sqlx::query_file_as!(User, "queries/users/get_by_email.sql", email.to_lowercase())
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
            data.email.map(|e| e.to_lowercase()),
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

fn hash_password(password: &str) -> argon2::password_hash::Result<String> {
    use argon2::{
        Argon2,
        password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
    };
    let salt = SaltString::generate(&mut OsRng);

    // Argon2 with default params (Argon2id v19)
    let argon2 = Argon2::default();

    // Hash password to PHC string ($argon2id$v=19$...)
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string();
    Ok(password_hash)
}

fn verify_password(password_hash: &str, password: &str) -> argon2::password_hash::Result<bool> {
    use argon2::{
        Argon2,
        password_hash::{PasswordHash, PasswordVerifier},
    };
    let parsed_hash = PasswordHash::new(password_hash)?;
    let res = Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok();
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fake::Fake;
    use fake::faker::internet::en::{SafeEmail, Username};
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::faker::name::en::{FirstName, LastName};
    use uuid::Uuid;

    fn create_fake_user() -> CreateUser {
        CreateUser {
            username: Username().fake(),
            email: SafeEmail().fake(),
            password: "Password123!".to_string(),
            first_name: Some(FirstName().fake()),
            last_name: Some(LastName().fake()),
            bio: Some(Sentence(1..5).fake()),
        }
    }

    fn create_fake_update_user() -> UpdateUser {
        UpdateUser {
            username: Some(Username().fake()),
            email: Some(SafeEmail().fake()),
            password: Some("NewPassword123!".to_string()),
            first_name: Some(FirstName().fake()),
            last_name: Some(LastName().fake()),
            bio: Some(Paragraph(1..3).fake()),
        }
    }

    #[sqlx::test]
    async fn test_create_user_success(pool: sqlx::PgPool) -> anyhow::Result<()> {
        sqlx::migrate!().run(&pool).await?;
        let storage = UsersStorage::new(pool).await?;

        let user_data = create_fake_user();
        let created_user = storage.create(user_data.clone()).await?;

        assert_eq!(created_user.username, user_data.username);
        assert_eq!(created_user.email, user_data.email.to_lowercase());
        assert_eq!(created_user.first_name, user_data.first_name);
        assert_eq!(created_user.last_name, user_data.last_name);
        assert_eq!(created_user.bio, user_data.bio);
        assert_ne!(created_user.id, Uuid::nil());
        let now = chrono::Utc::now();
        assert!(
            now.signed_duration_since(created_user.created_at)
                .num_seconds()
                < 60
        );

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_by_id_success(pool: sqlx::PgPool) -> anyhow::Result<()> {
        sqlx::migrate!().run(&pool).await?;
        let storage = UsersStorage::new(pool).await?;

        let user_data = create_fake_user();
        let created_user = storage.create(user_data).await?;

        let found_user = storage.get_by_id(created_user.id).await?;
        assert!(found_user.is_some());
        assert_eq!(found_user.unwrap().id, created_user.id);

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_by_id_not_found(pool: sqlx::PgPool) -> anyhow::Result<()> {
        sqlx::migrate!().run(&pool).await?;
        let storage = UsersStorage::new(pool).await?;

        let random_id = Uuid::new_v4();
        let found_user = storage.get_by_id(random_id).await?;
        assert!(found_user.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_by_email_success(pool: sqlx::PgPool) -> anyhow::Result<()> {
        sqlx::migrate!().run(&pool).await?;
        let storage = UsersStorage::new(pool).await?;

        let user_data = create_fake_user();
        let created_user = storage.create(user_data).await?;

        let found_user = storage.get_by_email(&created_user.email).await?;
        assert!(found_user.is_some());
        assert_eq!(found_user.unwrap().email, created_user.email);

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_by_email_case_insensitive(pool: sqlx::PgPool) -> anyhow::Result<()> {
        sqlx::migrate!().run(&pool).await?;
        let storage = UsersStorage::new(pool).await?;

        let mut user_data = create_fake_user();
        user_data.email = "Test@Example.COM".to_string();
        let _created_user = storage.create(user_data).await?;

        let found_user = storage.get_by_email("test@example.com").await?;
        assert!(found_user.is_some());
        assert_eq!(found_user.unwrap().email, "test@example.com");

        Ok(())
    }

    #[sqlx::test]
    async fn test_verify_user_success(pool: sqlx::PgPool) -> anyhow::Result<()> {
        sqlx::migrate!().run(&pool).await?;
        let storage = UsersStorage::new(pool).await?;

        let user_data = create_fake_user();
        let created_user = storage.create(user_data).await?;

        let is_valid = storage
            .verify_user(&created_user.email, "Password123!")
            .await?;
        assert!(is_valid);

        Ok(())
    }

    #[sqlx::test]
    async fn test_verify_user_wrong_password(pool: sqlx::PgPool) -> anyhow::Result<()> {
        sqlx::migrate!().run(&pool).await?;
        let storage = UsersStorage::new(pool).await?;

        let user_data = create_fake_user();
        let created_user = storage.create(user_data).await?;

        let is_valid = storage
            .verify_user(&created_user.email, "WrongPassword123!")
            .await?;
        assert!(!is_valid);

        Ok(())
    }

    #[sqlx::test]
    async fn test_verify_user_not_found(pool: sqlx::PgPool) -> anyhow::Result<()> {
        sqlx::migrate!().run(&pool).await?;
        let storage = UsersStorage::new(pool).await?;

        let is_valid = storage
            .verify_user("nonexistent@example.com", "Password123!")
            .await;
        assert!(is_valid.is_err());

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_users_empty(pool: sqlx::PgPool) -> anyhow::Result<()> {
        sqlx::migrate!().run(&pool).await?;
        let storage = UsersStorage::new(pool).await?;

        let search = UserSearch::default();
        let result = storage.list_users(search).await?;

        assert_eq!(result.users.len(), 0);
        assert_eq!(result.total_count, 0);
        assert_eq!(result.limit, 20);
        assert_eq!(result.offset, 0);

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_users_with_data(pool: sqlx::PgPool) -> anyhow::Result<()> {
        sqlx::migrate!().run(&pool).await?;
        let storage = UsersStorage::new(pool).await?;

        // Create multiple users
        for _ in 0..5 {
            let user_data = create_fake_user();
            storage.create(user_data).await?;
        }

        let search = UserSearch::default();
        let result = storage.list_users(search).await?;

        assert_eq!(result.users.len(), 5);
        assert_eq!(result.total_count, 5);
        assert_eq!(result.limit, 20);
        assert_eq!(result.offset, 0);

        // Verify ordering (newest first)
        for i in 1..result.users.len() {
            assert!(
                result.users[i].created_at <= result.users[i - 1].created_at,
                "Users should be ordered by created_at DESC"
            );
        }

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_users_pagination(pool: sqlx::PgPool) -> anyhow::Result<()> {
        sqlx::migrate!().run(&pool).await?;
        let storage = UsersStorage::new(pool).await?;

        // Create 5 users
        let mut created_emails = Vec::new();
        for _ in 0..5 {
            let user_data = create_fake_user();
            let email = user_data.email.clone();
            let _created_user = storage.create(user_data).await?;
            created_emails.push(email);
        }

        // First page (limit 2, offset 0)
        let search1 = UserSearch {
            search: None,
            limit: Some(2),
            offset: Some(0),
        };
        let result1 = storage.list_users(search1).await?;
        assert_eq!(result1.users.len(), 2);
        assert_eq!(result1.total_count, 5);
        assert_eq!(result1.limit, 2);
        assert_eq!(result1.offset, 0);

        // Second page (limit 2, offset 2)
        let search2 = UserSearch {
            search: None,
            limit: Some(2),
            offset: Some(2),
        };
        let result2 = storage.list_users(search2).await?;
        assert_eq!(result2.users.len(), 2);
        assert_eq!(result2.total_count, 5);

        // Verify no overlap between pages
        let page1_emails: Vec<String> = result1.users.iter().map(|u| u.email.clone()).collect();
        let page2_emails: Vec<String> = result2.users.iter().map(|u| u.email.clone()).collect();
        for email in &page1_emails {
            assert!(!page2_emails.contains(email));
        }

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_users_search_by_username(pool: sqlx::PgPool) -> anyhow::Result<()> {
        sqlx::migrate!().run(&pool).await?;
        let storage = UsersStorage::new(pool).await?;

        // Create users with predictable usernames
        let user1_data = CreateUser {
            username: "testuser123".to_string(),
            email: "test1@example.com".to_string(),
            password: "Password123!".to_string(),
            first_name: None,
            last_name: None,
            bio: None,
        };
        let user2_data = CreateUser {
            username: "othertest456".to_string(),
            email: "test2@example.com".to_string(),
            password: "Password123!".to_string(),
            first_name: None,
            last_name: None,
            bio: None,
        };

        storage.create(user1_data).await?;
        storage.create(user2_data).await?;

        // Search for "test" should return both users
        let search = UserSearch {
            search: Some("test".to_string()),
            limit: Some(20),
            offset: Some(0),
        };
        let result = storage.list_users(search).await?;
        assert_eq!(result.users.len(), 2);
        assert_eq!(result.total_count, 2);

        // Search for "123" should return only the first user
        let search = UserSearch {
            search: Some("123".to_string()),
            limit: Some(20),
            offset: Some(0),
        };
        let result = storage.list_users(search).await?;
        assert_eq!(result.users.len(), 1);
        assert_eq!(result.total_count, 1);
        assert!(result.users[0].username.contains("123"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_users_search_by_email(pool: sqlx::PgPool) -> anyhow::Result<()> {
        sqlx::migrate!().run(&pool).await?;
        let storage = UsersStorage::new(pool).await?;

        let user1_data = CreateUser {
            username: "user1".to_string(),
            email: "john.doe@example.com".to_string(),
            password: "Password123!".to_string(),
            first_name: None,
            last_name: None,
            bio: None,
        };
        let user2_data = CreateUser {
            username: "user2".to_string(),
            email: "jane.smith@test.org".to_string(),
            password: "Password123!".to_string(),
            first_name: None,
            last_name: None,
            bio: None,
        };

        storage.create(user1_data).await?;
        storage.create(user2_data).await?;

        // Search for "example" should return the first user
        let search = UserSearch {
            search: Some("example".to_string()),
            limit: Some(20),
            offset: Some(0),
        };
        let result = storage.list_users(search).await?;
        assert_eq!(result.users.len(), 1);
        assert_eq!(result.total_count, 1);
        assert!(result.users[0].email.contains("example"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_user_success(pool: sqlx::PgPool) -> anyhow::Result<()> {
        sqlx::migrate!().run(&pool).await?;
        let storage = UsersStorage::new(pool).await?;

        let user_data = create_fake_user();
        let created_user = storage.create(user_data).await?;

        let update_data = create_fake_update_user();
        let updated_user = storage.update(created_user.id, update_data.clone()).await?;

        assert!(updated_user.is_some());
        let user = updated_user.unwrap();
        assert_eq!(user.id, created_user.id);
        assert_eq!(user.username, update_data.username.unwrap());
        assert_eq!(user.email, update_data.email.unwrap().to_lowercase());
        assert_eq!(user.first_name, update_data.first_name);
        assert_eq!(user.last_name, update_data.last_name);
        assert_eq!(user.bio, update_data.bio);

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_user_not_found(pool: sqlx::PgPool) -> anyhow::Result<()> {
        sqlx::migrate!().run(&pool).await?;
        let storage = UsersStorage::new(pool).await?;

        let update_data = create_fake_update_user();
        let random_id = Uuid::new_v4();
        let updated_user = storage.update(random_id, update_data).await?;

        assert!(updated_user.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_user_success(pool: sqlx::PgPool) -> anyhow::Result<()> {
        sqlx::migrate!().run(&pool).await?;
        let storage = UsersStorage::new(pool).await?;

        let user_data = create_fake_user();
        let created_user = storage.create(user_data).await?;

        let deleted_id = storage.delete(created_user.id).await?;
        assert!(deleted_id.is_some());
        assert_eq!(deleted_id.unwrap(), created_user.id);

        // Verify user is actually deleted
        let found_user = storage.get_by_id(created_user.id).await?;
        assert!(found_user.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_user_not_found(pool: sqlx::PgPool) -> anyhow::Result<()> {
        sqlx::migrate!().run(&pool).await?;
        let storage = UsersStorage::new(pool).await?;

        let random_id = Uuid::new_v4();
        let deleted_id = storage.delete(random_id).await?;

        assert!(deleted_id.is_none());

        Ok(())
    }

    #[test]
    fn test_hash_password() {
        let password = "test_password_123!";
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();

        // Hashes should be different due to random salt
        assert_ne!(hash1, hash2);

        // Both hashes should be valid for the same password
        assert!(verify_password(&hash1, password).unwrap());
        assert!(verify_password(&hash2, password).unwrap());

        // Hashes should not work for different passwords
        assert!(!verify_password(&hash1, "wrong_password").unwrap());
    }

    #[test]
    fn test_verify_password() {
        let password = "test_password_123!";
        let hash = hash_password(password).unwrap();

        // Correct password should verify
        assert!(verify_password(&hash, password).unwrap());

        // Wrong password should not verify
        assert!(!verify_password(&hash, "wrong_password").unwrap());

        // Invalid hash should error
        assert!(verify_password("invalid_hash", password).is_err());
    }
}
