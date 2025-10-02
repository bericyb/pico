pub mod sql {
    use std::{
        collections::HashMap,
        error::Error,
        fs::{self, File},
        io::Read,
    };

    use chrono::DateTime;
    use postgres::{Client, NoTls};
    use regex::Regex;

    pub struct SQL {
        connection: Client,
        sprocs: HashMap<String, Sproc>,
    }

    pub struct Sproc {
        sql: String, // A parameterized version of the sql where the parameter names are replaced with indexes.
        parameters: Vec<String>, // A vector of parameter names in order of insertion in the sql statement
    }

    /// LMAO why do we worry about pool size if we're single threaded 0.0
    pub fn initialize_sql_service(conn_str: &String) -> Result<SQL, String> {
        let mut connection = match Client::connect(conn_str.as_str(), NoTls) {
            Ok(c) => c,
            Err(e) => return Err(format!("error connecting to database, {}", e)),
        };

        match migrate_db(&mut connection) {
            Ok(_) => {}
            Err(e) => return Err(format!("error migrating database, {}", e)),
        }

        let sprocs = match load_sprocs() {
            Ok(s) => s,
            Err(e) => return Err(format!("error loading sql sprocs: {}", e)),
        };

        return Ok(SQL { connection, sprocs });
    }

    /// Load up a hashmap of sql scripts to run at runtime
    /// In the future we should allow for nested folders of sprocs
    /// so that we make more complex apps.
    fn load_sprocs() -> Result<HashMap<String, Sproc>, Box<dyn std::error::Error>> {
        let dir_entries = match fs::read_dir("db/sprocs/") {
            Ok(e) => e,
            Err(e) => {
                return Err(format!(
                    "failed to read db/sprocs/ directory for stored sql scripts: {}",
                    e
                )
                .into());
            }
        };

        let mut sprocs = HashMap::new();
        for entry in dir_entries {
            let entry = entry?;
            if entry.path().is_file() {
                let mut f = File::open(entry.path())?;

                let mut sql = String::new();
                f.read_to_string(&mut sql)?;

                let mut parameters: Vec<String> = Vec::new();
                let r = Regex::new(r"\$(\w+)").unwrap();
                for params in r.find_iter(&sql) {
                    parameters.push(params.as_str().to_string());
                }

                for (i, param) in parameters.iter().enumerate() {
                    sql = sql.replace(format!("${}", param).as_str(), (i + 1).to_string().as_str());
                }

                let file_name = match entry.file_name().to_str() {
                    Some(n) => n.to_string(),
                    None => continue,
                };

                sprocs.insert(file_name, Sproc { sql, parameters });
            }
        }
        Ok(sprocs)
    }

    fn migrate_db(connection: &mut Client) -> Result<(), Box<dyn std::error::Error>> {
        // Check for migrations table
        match connection.execute("CREATE SCHEMA IF NOT EXISTS pico", &[]) {
            Ok(_) => {}
            Err(e) => return Err(format!("error creating internal pico schema: {}", e).into()),
        }

        match connection.execute("CREATE TABLE IF NOT EXISTS pico.migrations(id SERIAL PRIMARY KEY, name TEXT NOT NULL, applied_at TIMESTAMP)", &[]) {
            Ok(_) => {}
            Err(e) => return Err(format!("error creating pico migration tracker: {}", e).into()),
        }

        let last_migrated_at: DateTime<chrono::Utc> = match connection.query_opt(
            "SELECT applied_at FROM pico.migrations ORDER BY applied_at DESC LIMIT 1",
            &[],
        ) {
            Ok(Some(row)) => row.get("applied_at"),
            Ok(None) => DateTime::from_timestamp(0, 0).unwrap(),
            Err(e) => return Err(format!("db error while applying migrations: {}", e).into()),
        };

        let dir_entries = match fs::read_dir("db/migrations/") {
            Ok(des) => des,
            Err(e) => {
                return Err(format!(
                    "error finding migrations folder: {}\nIf using a custom directory to store migrations please define migrations = 'path/to/migrations/' in the DB table of your pico config",
                    e
                ).into());
            }
        };

        for entry in dir_entries {
            let entry = match entry {
                Ok(f) => f,
                Err(e) => {
                    return Err(
                        format!("failed to get entry while applying migrations {}", e).into(),
                    );
                }
            };

            let file_path: String = match entry.path().to_str() {
                Some(s) => s.to_string(),
                None => continue,
            };

            let file_path_splits: Vec<&str> = file_path.split('.').into_iter().collect();
            if file_path_splits.len() != 2 || file_path_splits[1] != "sql" {
                println!(
                    "file {} is not a sql migration file. migration files follow the format <timestamp>:<migration_name>.sql",
                    file_path
                );
                continue;
            }

            let file_name_splits: Vec<&str> = file_path_splits[0].split(':').into_iter().collect();
            if file_name_splits.len() != 2 {
                println!(
                    "file {} is not a properly named migration file. migration files follow the format <timestamp>:<migration_name>.sql",
                    file_path
                );
                continue;
            }
            let dt: DateTime<chrono::Utc> = match file_name_splits.get(0) {
                Some(dt_str) => match dt_str.parse() {
                    Ok(dt) => dt,
                    Err(e) => {
                        println!(
                            "file {} does not have a valid timestamp. migration files follow the format <timestamp>:<migration_name>.sql: {}",
                            file_path, e
                        );
                        continue;
                    }
                },
                None => {
                    println!(
                        "file {} is not a properly named migration file. migration files follow the format <timestamp>:<migration_name>.sql",
                        file_path
                    );
                    continue;
                }
            };

            if dt > last_migrated_at {
                match apply_migration(connection, file_path.to_string()) {
                    Ok(_) => {}
                    Err(e) => {
                        return Err(
                            format!("failed to apply migration {}: {}", file_path, e).into()
                        );
                    }
                }
            }
        }
        println!("Applied migrations!");

        Ok(())
    }

    fn apply_migration(client: &mut Client, file_path: String) -> Result<(), Box<dyn Error>> {
        let mut sql = String::new();
        let mut f = File::open(&file_path)?;

        f.read_to_string(&mut sql)?;

        match client.execute(&sql, &[]) {
            Ok(_) => {
                println!("applied migration {}", file_path);
                return Ok(());
            }
            Err(e) => {
                return Err(format!("failed to apply migration {}: {}", file_path, e).into());
            }
        }
    }
}
