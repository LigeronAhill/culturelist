-- Count users for pagination metadata
-- Parameters:
-- $1: search term (searches username, email, first_name, last_name, bio)
-- Returns total count of filtered users

SELECT COUNT(*) as total_count
FROM users
WHERE 
    $1::TEXT IS NULL OR $1::TEXT = '' OR
    username ILIKE '%' || $1::TEXT || '%' OR
    email ILIKE '%' || $1::TEXT || '%' OR
    COALESCE(first_name, '') ILIKE '%' || $1::TEXT || '%' OR
    COALESCE(last_name, '') ILIKE '%' || $1::TEXT || '%' OR
    COALESCE(bio, '') ILIKE '%' || $1::TEXT || '%';