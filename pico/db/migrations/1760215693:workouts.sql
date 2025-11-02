CREATE TABLE IF NOT EXISTS workouts (
    id SERIAL PRIMARY KEY,
    date TIMESTAMP NOT NULL,
    type VARCHAR(50) NOT NULL,
    duration INT NOT NULL, -- duration in minutes
    calories_burned INT,
    notes TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
