-- Ping counter table for tracking API usage
CREATE TABLE IF NOT EXISTS ping_counter (
    id SERIAL PRIMARY KEY,
    count INTEGER DEFAULT 0,
    last_ping TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);