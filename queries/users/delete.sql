-- Delete user by ID
-- Returns true if user was deleted, false if not found
DELETE FROM users
WHERE id = $1
RETURNING id;