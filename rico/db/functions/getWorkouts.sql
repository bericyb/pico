CREATE OR REPLACE FUNCTION getWorkouts()
RETURNS SETOF workouts AS $$
    SELECT * FROM workouts ORDER BY date DESC;
$$ LANGUAGE sql;
