INSERT INTO ping_counter (id, timestamp) VALUES (DEFAULT, NOW());
SELECT COUNT(*) FROM ping_counter;
