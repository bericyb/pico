CREATE OR REPLACE FUNCTION pong()
RETURNS TABLE(message text, count int, last_ping timestamp) AS $$
BEGIN
    -- Insert or update ping counter
    INSERT INTO ping_counter (id, count, last_ping)
    VALUES (1, 1, CURRENT_TIMESTAMP)
    ON CONFLICT (id)
    DO UPDATE SET 
        count = ping_counter.count + 1,
        last_ping = CURRENT_TIMESTAMP;
    
    -- Return the current state
    RETURN QUERY
    SELECT 
        'pong'::text as message,
        ping_counter.count,
        ping_counter.last_ping
    FROM ping_counter
    WHERE ping_counter.id = 1;
END;
$$ LANGUAGE plpgsql;