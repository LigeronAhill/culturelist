-- Create a new user
-- Returns the created user record
INSERT INTO users (username, email, password, first_name, last_name, bio)
  VALUES ($1, $2, $3, $4, $5, $6)
RETURNING
  id, username, email, first_name, last_name, bio, created_at;

