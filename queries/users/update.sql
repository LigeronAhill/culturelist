-- Update user by ID
-- Returns updated user record
UPDATE users 
SET 
    username = COALESCE($2, username),
    email = COALESCE($3, email),
    password = COALESCE($4, password),
    first_name = COALESCE($5, first_name),
    last_name = COALESCE($6, last_name),
    bio = COALESCE($7, bio)
WHERE id = $1
RETURNING id, username, email, first_name, last_name, bio, created_at;