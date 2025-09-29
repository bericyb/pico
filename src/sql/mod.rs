pub mod sql {
    use std::{collections::{HashMap, HashSet}, fs, time};

    use postgres::{Client, NoTls};

    pub struct SQL {
        connection: Client,
        sprocs: HashMap<String, Sproc>,
    }

    pub struct Sproc {
        sql: String,
    }

    /// LMAO why do we worry about pool size if we're single threaded 0.0
    pub fn initialize_sql_service(conn_str: String, _pool_size: usize) -> Result<SQL, String> {
        let connection = match Client::connect(conn_str.as_str(), NoTls) {
            Ok(c) => c,
            Err(e) => return Err(format!("error connecting to database, {}", e)),
        };

        // Do migrations
        //
        // Check for migration table
        match connection.execute("CREATE SCHEMA IF NOT EXISTS pico", &[]) {
            Ok(_) => {}
            Err(e) => return Err(format!("error creating internal pico schema: {}", e)),
        }

        match connection.execute("CREATE TABLE IF NOT EXISTS pico.migrations(id SERIAL PRIMARY KEY, name TEXT NOT NULL, applied_at TIMESTAMP)", &[]) {
            Ok(_) => {}
            Err(e) => return Err(format!("error creating pico migration tracker: {}", e)),
        }

        let dir_entries = match fs::read_dir("db/migrations/") {
            Ok(des) => des,
            Err(e) => return Err(format!("error finding migrations folder: {}\nIf using a custom directory to store migrations please define migrations = 'path/to/migrations/' in the DB table of your pico config", e)),
        }

        for entry in dir_entries {
            let entry = match entry {
                Ok(f) => f,
            Err(e) => return Err(format!("", e)),
            };

            let file_path = match entry.path().to_str() {
                Some(fp) => fp,
                None => continue,
            }

            let file_splits: Vec<&str>= file_path.split('.').into_iter().collect();
            if file_splits.len() != 2 || file_splits[1] != "sql" {
                continue;
            }

            let ts_file_name: Vec<&str> = file_splits[0].split(':').into_iter().collect();



            

        }


        let latest_mig_time = match connection.query("SELECT applied_at FROM pico.migrations ORDER BY applied_at DESC LIMIT 1", &[]) {
            Ok(l) => {
                if l.len() == 0 {
                    time::UNIX_EPOCH
                } else {
                        l[0].get(0)
                }
            },
            Err(e) => return Err(format!("error creating pico migration tracker: {}", e)),
        }




        for migration in migrations {
        }

        let sprocs = HashMap::new();

        return Err("Error setting up sql database");
    }
}
