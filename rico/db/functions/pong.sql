CREATE OR REPLACE FUNCTION pong ()
RETURNS TABLE(ping_count int) AS $$
	INSERT INTO ping_counter DEFAULT VALUES;
	SELECT COUNT(*) AS ping_count FROM ping_counter;
$$ LANGUAGE sql;
