CREATE OR REPLACE FUNCTION authenticate_user(user_email text, user_password text)
RETURNS TABLE(id int, email text) AS $$
BEGIN
    RETURN QUERY
    SELECT users.id, users.email
    FROM users
    WHERE users.email = user_email 
    AND users.password_hash = crypt(user_password, users.password_hash);
END;
$$ LANGUAGE plpgsql;