pub mod sql {
    use std::{
        collections::HashMap,
        error::Error,
        fs::{self, File},
        io::Read,
    };

    use chrono::{DateTime, NaiveDate, NaiveDateTime};
    use postgres::{Client, NoTls, Row, types::ToSql};
    use serde_json::{Value, json};
    use sqlparser::{
        ast::{CreateFunction, Statement},
        dialect::PostgreSqlDialect,
        parser::Parser,
    };

    use crate::http::http::ResponseCode;

    pub const SQL_FUNCTION_TEMPLATE: &str = "CREATE OR REPLACE FUNCTION {name}(example_parameter int)\nRETURNS TABLE(example_result text) AS $$\n\t<SQL STATEMENTS>;\n$$ LANGUAGE sql;";

    pub struct SQL {
        pub connection: Client,
        pub functions: HashMap<String, Function>,
    }

    pub struct Function {
        pub fn_call_statement: String, // SQL statement to execute a function with indexed parameters
        pub parameters: Vec<String>, // Parameter names in order of insertion in the fn_call_statement
    }

    impl Function {
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
            // 1. Convert Vec<Value> to Vec<Box<dyn ToSql + Sync>> via explicit matching
            let boxed_params: Vec<Box<dyn ToSql + Sync>> = ingestion_params.clone()
            .into_iter() // Consume the Vec to take ownership of each Value
            .map(|v| {
                match v {
                    // Handle String values (maps to TEXT/VARCHAR)
                    Value::String(s) => {
                        Box::new(s) as Box<dyn ToSql + Sync>
                    }
                    // Handle Number values (maps to INTEGER, BIGINT, NUMERIC)
                    Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            // Treat as an integer
                            Box::new(i) as Box<dyn ToSql + Sync>
                        } else if let Some(f) = n.as_f64() {
                            // Treat as a floating point number
                            Box::new(f) as Box<dyn ToSql + Sync>
                        } else {
                            // Catch numbers that are too large for i64/f64 (should be rare)
                            panic!("JSON number too large or complex for simple SQL type.")
                        }
                    }
                    // Handle Boolean values (maps to BOOLEAN/BOOL)
                    Value::Bool(b) => {
                        Box::new(b) as Box<dyn ToSql + Sync>
                    }
                    // Handle Null values (maps to SQL NULL)
                    Value::Null => {
                        // Option<T> is how you send NULL, using a placeholder type like &str
                        Box::new(None::<&str>) as Box<dyn ToSql + Sync>
                    }
                    // Handle Array and Object (maps to JSONB or fails for simple types)
                    // If the column expects TEXT/INT/BOOL, these are invalid
                    _ => {
                        // If you hit this, the parameter is likely invalid for a simple column
                        panic!("Unsupported JSON type {:?} for simple database column. Arrays/Objects must be handled as JSONB.", v)
                    }
                }
            })
            .collect();

            // 2. Map to references for the final slice
            let param_refs: Vec<&(dyn ToSql + Sync)> =
                boxed_params.iter().map(|b| b.as_ref()).collect();

            // 3. Get the final slice
            let params_slice: &[&(dyn ToSql + Sync)] = param_refs.as_slice();

            // fn_call_statement looks like the following when we execute it here.
            // SELECT function_name($1, $2);
            println!("Executing SQL: {}", &self.fn_call_statement);
            let res = match client.query(&self.fn_call_statement, &params_slice) {
                Ok(r) => r,
                Err(e) => {
                    println!(
                        "error executing sql function with: {} : Input: {:#?} : Error: {}",
                        &self.fn_call_statement, &ingestion_params, e
                    );
                    return Err(ResponseCode::InternalError);
                }
            };

            let mut results: Vec<Value> = vec![];
            for row in res {
                let json_row = row_to_json(&row);
                results.push(json_row);
            }

            if results.len() == 0 {
                return Ok(Value::Null);
            } else if results.len() == 1 {
                return Ok(results[0].clone());
            } else {
                return Ok(Value::Array(results));
            }
        }
    }

    pub fn initialize_sql_service(conn_str: &String) -> Result<SQL, String> {
        let mut connection = match Client::connect(conn_str.as_str(), NoTls) {
            Ok(c) => c,
            Err(e) => return Err(format!("error connecting to database, {}", e)),
        };

        match migrate_db(&mut connection) {
            Ok(_) => {}
            Err(e) => return Err(format!("error migrating database, {}", e)),
        }

        let functions = match load_functions(&mut connection) {
            Ok(s) => s,
            Err(e) => return Err(format!("error loading sql functions: {}", e)),
        };

        return Ok(SQL {
            connection,
            functions,
        });
    }

    /// Load up a hashmap of sql scripts to run at runtime
    /// In the future we should allow for nested folders of functions
    /// so that we make more complex apps.
    fn load_functions(
        client: &mut Client,
    ) -> Result<HashMap<String, Function>, Box<dyn std::error::Error>> {
        let dir_entries = match fs::read_dir("functions/") {
            Ok(e) => e,
            Err(e) => {
                return Err(format!(
                    "failed to read functions/ directory for stored sql scripts: {}",
                    e
                )
                .into());
            }
        };

        let mut functions = HashMap::new();
        for entry in dir_entries {
            let entry = entry?;
            if entry.path().is_file() {
                let mut f = File::open(entry.path())?;

                let mut sql = String::new();
                f.read_to_string(&mut sql)?;

                let file_name = match entry.file_name().to_str() {
                    Some(n) => n.strip_suffix(".sql").unwrap_or(n).to_string(),
                    None => continue,
                };

                let dialect = PostgreSqlDialect {};
                let statements: Vec<Statement> = match Parser::parse_sql(&dialect, &sql) {
                    Ok(s) => s,
                    Err(e) => {
                        return Err(format!(
                            "error parsing sql in function {:#?}: {}",
                            entry.file_name().to_os_string(),
                            e
                        )
                        .into());
                    }
                };

                // At this point we should only have one statement for a function
                // if not, we throw an error and recommend the user put all statements
                // inside the one function
                if statements.len() != 1 {
                    return Err(format!(
                        "multiple statements found for a single function: {:#?}. Pico only supports one function creation statement per function file. If you need multiple statements, please declare them all within the single function or create a new function.",
                        entry.file_name().to_os_string()
                    ).into());
                }

                let statement = statements.get(0).unwrap();
                let function: &CreateFunction = match statement {
                    Statement::CreateFunction(f) => f,
                    _ => {
                        return Err(format!(
                            "sql found in {:#?} is not a CREATE FUNCTION declaration.",
                            entry.file_name().to_os_string()
                        )
                        .into());
                    }
                };

                // Use the file name as function name (already extracted above)
                let function_name = &file_name;
                
                // Drop the function if it exists (ignore errors if it doesn't exist)
                let drop_sql = format!("DROP FUNCTION IF EXISTS {} CASCADE", function_name);
                match client.execute(&drop_sql, &[]) {
                    Ok(_) => println!("Dropped existing function: {}", function_name),
                    Err(e) => println!("Note: Could not drop function {} (may not exist): {}", function_name, e),
                }

                // Create the new function
                match client.execute(&function.to_string(), &[]) {
                    Ok(_) => println!("Created function: {}", function_name),
                    Err(e) => {
                        return Err(format!(
                            "failed to create sql function {:#?}: error {}",
                            entry.file_name().to_str(),
                            e
                        )
                        .into());
                    }
                }

                let mut parameters: Vec<String> = vec![];
                function
                    .args
                    .clone()
                    .unwrap_or(vec![])
                    .iter()
                    .for_each(|arg| match &arg.name {
                        Some(a) => parameters.push(a.value.clone()),
                        None => {
                            println!(
                                "parameter for function {:#?} found with name?",
                                entry.file_name().to_str()
                            );
                        }
                    });

                let mut fn_call_statement = format!("SELECT * FROM {}(", file_name);

                for (i, _param) in parameters.iter().enumerate() {
                    fn_call_statement = fn_call_statement + format!("${}", i + 1).as_str();
                    if i < parameters.len() - 1 {
                        fn_call_statement = fn_call_statement + ", "
                    }
                }
                fn_call_statement = fn_call_statement + ");";

                functions.insert(
                    file_name,
                    Function {
                        fn_call_statement,
                        parameters,
                    },
                );
            }
        }
        Ok(functions)
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

        // assuming connection is established
        let last_migrated_at: i64 = match connection.query_opt(
            "SELECT applied_at FROM pico.migrations ORDER BY applied_at DESC LIMIT 1",
            &[],
        ) {
            Ok(Some(row)) => {
                let ts: NaiveDateTime = row.get("applied_at"); // get as NaiveDateTime
                ts.and_utc().timestamp() // convert to i64 Unix epoch seconds
            }
            Ok(None) => 0, // default to epoch
            Err(e) => return Err(format!("db error while applying migrations: {}", e).into()),
        };
        // TODO: allow for custom migrations directory in pico config
        let dir_entries = match fs::read_dir("migrations/") {
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

            let file_name_splits: Vec<&str> = file_name.split(':').into_iter().collect();
            if file_name_splits.len() != 2 {
                println!(
                    "file {} is not a properly named migration file. migration files follow the format <timestamp>:<migration_name>.sql",
                    file_path
                );
                continue;
            }
            let dt: i64 = match file_name_splits.get(0) {
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
                match apply_migration(connection, file_path.to_string(), dt) {
                    Ok(_) => {}
                    Err(e) => {
                        return Err(
                            format!("failed to apply migration {}: {}", file_path, e).into()
                        );
                    }
                }
            }
        }
        println!("Migrations applied!");

        Ok(())
    }

    fn apply_migration(
        client: &mut Client,
        file_path: String,
        migration_time: i64,
    ) -> Result<(), Box<dyn Error>> {
        let mut sql = String::new();
        let mut f = File::open(&file_path)?;

        f.read_to_string(&mut sql)?;
        if sql.trim().is_empty() {
            println!("skipping empty migration {}", file_path);
            return Ok(());
        }

        match client.execute(&sql, &[]) {
            Ok(_) => {
                println!("applied migration {}", file_path);
            }
            Err(e) => {
                return Err(format!("failed to apply migration {}: {}", file_path, e).into());
            }
        }

        println!("timestamp passed in {}", migration_time);
        let migration_time: NaiveDateTime = DateTime::from_timestamp(migration_time, 0)
            .unwrap()
            .naive_utc();
        match client.execute(
            "INSERT INTO pico.migrations (name, applied_at) VALUES ($1, $2)",
            &[&file_path, &migration_time],
        ) {
            Ok(_) => {
                return Ok(());
            }
            Err(e) => {
                return Err(format!("failed to track migration {}: {}", file_path, e).into());
            }
        }
    }

    fn row_to_json(row: &Row) -> Value {
        let mut obj = serde_json::Map::new();

        for (idx, column) in row.columns().iter().enumerate() {
            let val: Value = match column.type_().name() {
                "int2" => match row.try_get::<_, i16>(idx) {
                    Ok(v) => json!(v),
                    Err(e) => {
                        println!("Error reading int2 column '{}': {:?}", column.name(), e);
                        Value::Null
                    }
                },
                "int4" => match row.try_get::<_, i32>(idx) {
                    Ok(v) => json!(v),
                    Err(e) => {
                        println!("Error reading int4 column '{}': {:?}", column.name(), e);
                        Value::Null
                    }
                },
                "int8" => match row.try_get::<_, i64>(idx) {
                    Ok(v) => json!(v),
                    Err(e) => {
                        println!("Error reading int8 column '{}': {:?}", column.name(), e);
                        Value::Null
                    }
                },
                "float4" => match row.try_get::<_, f32>(idx) {
                    Ok(v) => json!(v),
                    Err(e) => {
                        println!("Error reading float4 column '{}': {:?}", column.name(), e);
                        Value::Null
                    }
                },
                "float8" => match row.try_get::<_, f64>(idx) {
                    Ok(v) => json!(v),
                    Err(e) => {
                        println!("Error reading float8 column '{}': {:?}", column.name(), e);
                        Value::Null
                    }
                },
                "bool" => match row.try_get::<_, bool>(idx) {
                    Ok(v) => json!(v),
                    Err(e) => {
                        println!("Error reading bool column '{}': {:?}", column.name(), e);
                        Value::Null
                    }
                },
                "text" | "varchar" | "char" => match row.try_get::<_, String>(idx) {
                    Ok(v) => json!(v),
                    Err(e) => {
                        println!("Error reading string column '{}': {:?}", column.name(), e);
                        Value::Null
                    }
                },
                "uuid" => match row.try_get::<_, String>(idx) {
                    Ok(v) => json!(v), // UUID as string; safer for postgres 0.19.11
                    Err(e) => {
                        println!("Error reading uuid column '{}': {:?}", column.name(), e);
                        Value::Null
                    }
                },
                "date" => match row.try_get::<_, NaiveDate>(idx) {
                    Ok(v) => json!(v.to_string()),
                    Err(e) => {
                        println!("Error reading date column '{}': {:?}", column.name(), e);
                        Value::Null
                    }
                },
                "timestamp" | "timestamptz" => match row.try_get::<_, NaiveDateTime>(idx) {
                    Ok(v) => json!(v.to_string()),
                    Err(e) => {
                        println!(
                            "Error reading timestamp column '{}': {:?}",
                            column.name(),
                            e
                        );
                        Value::Null
                    }
                },
                other => {
                    println!(
                        "Unsupported column type '{}' for column '{}'",
                        other,
                        column.name()
                    );
                    Value::Null
                }
            };

            obj.insert(column.name().to_string(), val);
        }

        Value::Object(obj)
    }
}
