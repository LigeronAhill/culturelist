-- List users with pagination and search
-- Parameters:
-- $1: search term (searches username, email, first_name, last_name, bio)
-- $2: limit (number of records per page)
-- $3: offset (pagination offset)
-- Returns paginated user records ordered by created_at DESC

SELECT 
    id,
    username,
    email,
    first_name,
    last_name,
    bio,
    created_at
FROM users
WHERE 
    $1::TEXT IS NULL OR $1::TEXT = '' OR
    username ILIKE '%' || $1::TEXT || '%' OR
    email ILIKE '%' || $1::TEXT || '%' OR
    COALESCE(first_name, '') ILIKE '%' || $1::TEXT || '%' OR
    COALESCE(last_name, '') ILIKE '%' || $1::TEXT || '%' OR
    COALESCE(bio, '') ILIKE '%' || $1::TEXT || '%'
ORDER BY created_at DESC
LIMIT $2 OFFSET $3;