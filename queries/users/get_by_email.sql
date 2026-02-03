-- Get user by email
-- Returns user record or null if not found
SELECT id, username, email, first_name, last_name, bio, created_at
FROM users
WHERE email = $1;