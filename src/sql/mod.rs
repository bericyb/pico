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
    use serde_json::Value;

    use crate::http::http::ResponseCode;

    pub struct SQL {
        pub connection: Client,
        pub sprocs: HashMap<String, Sproc>,
    }

    pub struct Sproc {
        pub sql: String, // A parameterized version of the sql where the parameter names are replaced with indexes.
        pub parameters: Vec<String>, // A vector of parameter names in order of insertion in the sql statement
    }

    impl Sproc {
        pub fn execute(
            &self,
            client: &mut Client,
            input: HashMap<String, Value>,
        ) -> Result<Value, ResponseCode> {
            let mut ingestion_params = vec![];
            for param in self.parameters.clone() {
                match input.get(&param) {
                    Some(p) => ingestion_params.push(p.clone()),
                    None => return Err(ResponseCode::BadRequest),
                }
            }

            let ingestion_bytes = match serde_json::to_vec(&ingestion_params) {
                Ok(ib) => ib,
                Err(e) => {
                    println!("failure converting to bytes before executing sproc {}", e);
                    return Err(ResponseCode::InternalError);
                }
            };

            let res = match client.query(&self.sql, &[&ingestion_bytes]) {
                Ok(r) => r,
                Err(e) => {
                    println!(
                        "error querying database with sproc: {} : Input: {:?} : Error: {}",
                        &self.sql, ingestion_bytes, e
                    );
                    return Err(ResponseCode::InternalError);
                }
            };

            let mut results: Vec<Value> = vec![];
            for row in res {
                let mut json_row = serde_json::Map::new();
                for col in row.columns() {
                    let name = col.name();
                    let value: Value = match row.try_get(name) {
                        Ok(val) => val,
                        Err(e) => {
                            println!("error getting column {} from result row: {}", name, e);
                            continue;
                        }
                    };
                    json_row.insert(name.to_string(), value);
                }
                results.push(Value::Object(json_row));
            }

            if results.len() == 0 {
                return Ok(Value::Null);
            } else if results.len() == 1 {
                return Ok(results[1].clone());
            } else {
                return Ok(Value::Array(results));
            }
        }
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

        // TODO: allow for custom migrations directory in pico config
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

            if entry.path().extension() != Some(std::ffi::OsStr::new("sql")) {
                println!(
                    "file {} is not a sql migration file. migration files follow the format <timestamp>:<migration_name>.sql",
                    file_path
                );
                continue;
            }

            let file_name: String = match entry.file_name().to_str() {
                Some(s) => s.to_string(),
                None => {
                    println!(
                        "file {} is not a properly named migration file. migration files follow the format <timestamp>:<migration_name>.sql",
                        file_path
                    );
                    continue;
                }
            };

            let file_name_splits: Vec<&str> = file_name.split('%').into_iter().collect();
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
                        println!("{}", dt_str);
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
