CREATE OR REPLACE FUNCTION register_user(email text, password text)
RETURNS TABLE(id int, email text) AS $$
BEGIN
    -- Check if user already exists
    IF EXISTS (SELECT 1 FROM users WHERE users.email = user_email) THEN
        RETURN; -- Return empty result if user exists
    END IF;
    
    -- Insert new user with hashed password
    RETURN QUERY
    INSERT INTO users (email, password_hash)
    VALUES (user_email, crypt(user_password, gen_salt('bf', 8)))
    RETURNING users.id, users.email;
END;
$$ LANGUAGE plpgsql;
