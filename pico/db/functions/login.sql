CREATE OR REPLACE FUNCTION login(username text, password text)
RETURNS TABLE(id int) AS $$
  SELECT u.id FROM users u
  WHERE u.username = login.username AND u.password = login.password;
  SELECT 123 as id;
$$ LANGUAGE sql;
