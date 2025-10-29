# SQL Handler

The SQL handler is the core of Pico's database interaction. It executes SQL functions stored in your `functions/` directory and returns the results to the next handler in the pipeline.

## What is SQL for?

The SQL handler bridges your Lua application logic with your PostgreSQL database. Use it to:
- Execute database queries and modifications
- Call stored procedures and functions
- Perform complex database operations
- Return structured data from your database

## Structure

The SQL handler expects a string that references a `.sql` file in your `functions/` directory:

```lua
SQL = "function_name.sql"
```

## SQL Function Format

Your SQL files should contain PostgreSQL function definitions that follow this pattern:

```sql
CREATE OR REPLACE FUNCTION function_name(param1 type, param2 type, ...)
RETURNS TABLE(column1 type, column2 type, ...) AS $$
    -- Your SQL logic here
$$ LANGUAGE sql;
```

## Examples

### Simple Query Function
```sql
-- functions/get_user_count.sql
CREATE OR REPLACE FUNCTION get_user_count()
RETURNS TABLE(count bigint) AS $$
    SELECT COUNT(*) as count FROM users;
$$ LANGUAGE sql;
```

Used in a route:
```lua
{
    SQL = "get_user_count.sql"
}
```

### Function with Parameters
```sql
-- functions/get_user_by_id.sql
CREATE OR REPLACE FUNCTION get_user_by_id(user_id int)
RETURNS TABLE(id int, username text, email text) AS $$
    SELECT u.id, u.username, u.email 
    FROM users u 
    WHERE u.id = get_user_by_id.user_id;
$$ LANGUAGE sql;
```

Used with PREPROCESS to pass parameters:
```lua
{
    PREPROCESS = function(req)
        return { user_id = req.id }
    end,
    SQL = "get_user_by_id.sql"
}
```

### Insert with Return
```sql
-- functions/create_user.sql
CREATE OR REPLACE FUNCTION create_user(username text, email text, password_hash text)
RETURNS TABLE(id int, created_at timestamp) AS $$
    INSERT INTO users (username, email, password_hash, created_at)
    VALUES (create_user.username, create_user.email, create_user.password_hash, NOW())
    RETURNING id, created_at;
$$ LANGUAGE sql;
```

### Complex Function with Multiple Operations
```sql
-- functions/login.sql
CREATE OR REPLACE FUNCTION login(username text, password text)
RETURNS TABLE(id int, username text, last_login timestamp) AS $$
    UPDATE users 
    SET last_login = NOW() 
    WHERE users.username = login.username AND users.password = login.password;
    
    SELECT u.id, u.username, u.last_login 
    FROM users u
    WHERE u.username = login.username AND u.password = login.password;
$$ LANGUAGE sql;
```

### Function that Performs Side Effects
```sql
-- functions/pong.sql
CREATE OR REPLACE FUNCTION pong()
RETURNS TABLE(ping_count int) AS $$
    INSERT INTO ping_counter DEFAULT VALUES;
    SELECT COUNT(*) AS ping_count FROM ping_counter;
$$ LANGUAGE sql;
```

## Parameter Binding

Pico automatically binds request data to function parameters by name. If your PREPROCESS handler returns:
```lua
{ username = "john", email = "john@example.com" }
```

It will be passed to a function like:
```sql
CREATE OR REPLACE FUNCTION some_function(username text, email text)
```

## Return Types

SQL functions can return:

### Single Row
```sql
RETURNS TABLE(id int, name text)
```
Returns an object: `{ id = 123, name = "John" }`

### Multiple Rows
```sql
RETURNS TABLE(id int, name text)
```
Returns an array: `[{ id = 123, name = "John" }, { id = 124, name = "Jane" }]`

### Single Value
```sql
RETURNS int
```
Returns the value directly: `123`

## Error Handling

PostgreSQL errors are automatically caught and returned as HTTP errors. You can also use PostgreSQL's `RAISE` statement:

```sql
CREATE OR REPLACE FUNCTION create_user(username text, email text)
RETURNS TABLE(id int) AS $$
BEGIN
    IF username IS NULL OR LENGTH(username) < 3 THEN
        RAISE EXCEPTION 'Username must be at least 3 characters';
    END IF;
    
    INSERT INTO users (username, email) VALUES (username, email) RETURNING id;
END;
$$ LANGUAGE plpgsql;
```

## Best Practices

1. **Use Descriptive Names**: Function names should clearly indicate their purpose
2. **Parameter Validation**: Validate inputs within your SQL functions
3. **Return Meaningful Data**: Structure your returns to match what your application needs
4. **Handle Edge Cases**: Consider what happens with invalid or missing data
5. **Use Transactions**: For operations that modify multiple tables
6. **Leverage PostgreSQL Features**: Use CTEs, window functions, and other PostgreSQL capabilities

## Integration with Other Handlers

The SQL handler receives input from PREPROCESS and sends output to POSTPROCESS:

```lua
{
    PREPROCESS = function(req)
        return { email = string.lower(req.email) }
    end,
    SQL = "find_user_by_email.sql", -- Gets { email = "user@example.com" }
    POSTPROCESS = function(resp)
        if resp.id then
            return "Found user: " .. resp.username
        else
            return "User not found"
        end
    end
}
```

## Database Migrations

SQL functions work alongside your database migrations. Create your tables in migration files and your functions in the `functions/` directory:

```
Application/
├── migrations/
│   └── 001_create_users.sql
└── functions/
    ├── create_user.sql
    ├── get_user.sql
    └── update_user.sql
```

This separation keeps your schema changes and business logic organized.