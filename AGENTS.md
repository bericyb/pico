# AGENTS.md - AI Assistant Guide for Pico Framework

This guide helps AI assistants understand and effectively work with Pico projects. Pico is a minimalistic full-stack web framework that combines SQL, Lua, and HTTP to create complete web applications with minimal code.

## Framework Overview

Pico is a PostgreSQL-based web framework where:
- **Routes** are defined as Lua tables in `config.lua`
- **Business logic** is implemented as PostgreSQL functions in the `functions/` directory
- **Database schema** is managed through timestamped migrations in `migrations/`
- **Static files** are served from the `public/` directory
- **Request pipeline** consists of: PREPROCESS → SQL → POSTPROCESS → SETJWT → VIEW

## Project Structure

Every Pico project follows this structure:
```
Project/
├── config.lua              # Main configuration and route definitions
├── functions/               # PostgreSQL function files (.sql)
│   ├── authenticate_user.sql
│   ├── create_user.sql
│   └── pong.sql
├── migrations/              # Database schema migrations (timestamp:name.sql)
│   ├── 1760204644:enable_pgcrypto.sql
│   ├── 1760215693:create_users_table.sql
│   └── 1760820197:ping_counter.sql
└── public/                  # Static assets (CSS, JS, images, HTML)
    ├── index.html
    └── styles.css
```

## Core Concepts

### 1. Routes and Handlers
Routes are defined in `config.lua` with HTTP methods and handler pipelines:

```lua
ROUTES = {
    ['users/:id'] = {
        GET = {
            PREPROCESS = function(req, jwt) -- Optional: transform request
                req.user_id = tonumber(req.id)
                return req
            end,
            SQL = "get_user_by_id.sql",     -- Required: SQL function to execute
            POSTPROCESS = function(resp, jwt) -- Optional: transform response
                resp.full_name = resp.first_name .. " " .. resp.last_name
                return resp
            end,
            SETJWT = function(resp, jwt)    -- Optional: manage authentication
                return jwt -- or nil to clear, or new claims
            end,
            VIEW = {                        -- Optional: render HTML
                { TYPE = "MARKDOWN" }
            }
        }
    }
}
```

### 2. Parameter Mapping
**Critical**: Request parameters must exactly match SQL function parameter names:
- URL parameters: `/users/:user_id` → `user_id` parameter
- Query params: `?email=test@example.com` → `email` parameter  
- Form/JSON data: `{"username": "john"}` → `username` parameter

### 3. SQL Functions
All database operations use PostgreSQL functions in `functions/`:

```sql
-- functions/create_user.sql
CREATE OR REPLACE FUNCTION create_user(username text, email text, password_hash text)
RETURNS TABLE(id int, created_at timestamp) AS $$
    INSERT INTO users (username, email, password_hash, created_at)
    VALUES (create_user.username, create_user.email, create_user.password_hash, NOW())
    RETURNING id, created_at;
$$ LANGUAGE sql;
```

### 4. Migrations
Database changes are managed through timestamped migration files:
```sql
-- migrations/1760820197:create_users_table.sql
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(50) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

## Development Guidelines for AI Assistants

### When Adding Features

1. **Always check parameter mapping**: Ensure request parameter names match SQL function parameters exactly
2. **Create migrations first**: Define database schema before writing functions
3. **Write SQL functions**: Implement business logic in PostgreSQL
4. **Define routes**: Connect HTTP endpoints to SQL functions via `config.lua`
5. **Test the complete pipeline**: Verify PREPROCESS → SQL → POSTPROCESS flow

### Database Operations

**Creating new tables:**
```bash
# Use admin.lua to generate migration
lua admin.lua migrate "create_posts_table"
```

**Adding SQL functions:**
```bash
# Use admin.lua to generate function template
lua admin.lua function "get_user_posts"
```

### Common Patterns

**Authentication route:**
```lua
['login'] = {
    POST = {
        PREPROCESS = function(req)
            req.email = string.lower(req.email) -- normalize email
            return req
        end,
        SQL = "authenticate_user.sql",
        POSTPROCESS = function(resp)
            if resp.id then
                return "Login successful"
            else
                return "Invalid credentials"
            end
        end,
        SETJWT = function(resp, jwt)
            if resp.id then
                return {
                    user_id = resp.id,
                    username = resp.username,
                    login_time = os.time()
                }
            end
            return nil -- Clear JWT on failed login
        end,
        VIEW = {
            { TYPE = "MARKDOWN" },
            { TYPE = "LINKS", LINKS = {{ value = "", label = "Home" }}}
        }
    }
}
```

**Data listing with forms:**
```lua
['users'] = {
    GET = {
        SQL = "get_all_users.sql",
        VIEW = {
            { TYPE = "TABLE" },
            { TYPE = "LINKS", LINKS = {{ value = "users/new", label = "Add User" }}}
        }
    },
    POST = {
        SQL = "create_user.sql",
        POSTPROCESS = function(resp)
            if resp.id then
                return "User created successfully"
            else
                return "Failed to create user"
            end
        end,
        VIEW = {
            { TYPE = "MARKDOWN" }
        }
    }
}
```

### Security Best Practices

1. **Input validation in PREPROCESS**: Validate and sanitize inputs before SQL
2. **Use parameterized functions**: Never concatenate user input into SQL strings
3. **JWT claims**: Only store non-sensitive data in JWT tokens
4. **Password hashing**: Use pgcrypto extension with `crypt()` and `gen_salt()`
5. **Static file security**: Only serve files from `public/` directory

### Error Handling

**In Lua handlers:**
```lua
PREPROCESS = function(req)
    if not req.email or req.email == "" then
        error("Email is required")
    end
    return req
end
```

**In SQL functions:**
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

### File Operations

**Reading project files:** Use `Read` tool to examine `config.lua`, migration files, and SQL functions

**Editing routes:** Use `Edit` tool to modify `config.lua` route definitions

**Creating migrations:** Use `Write` tool to create new migration files with timestamp format `{unix_timestamp}:{name}.sql`

**Creating SQL functions:** Use `Write` tool to add new `.sql` files in `functions/` directory

### Testing and Debugging

1. **Test parameter flow**: Verify request data reaches SQL functions correctly
2. **Check SQL syntax**: Ensure PostgreSQL functions are valid
3. **Validate JWT flow**: Confirm authentication state management
4. **Test static files**: Verify assets are accessible from `public/`

## View System

Pico includes a declarative view system for rapid frontend development:

```lua
VIEW = {
    {
        TYPE = "POSTFORM",
        TITLE = "Create User",
        TARGET = "/users",
        FIELDS = {
            { id = "username", type = "text", label = "Username" },
            { id = "email", type = "email", label = "Email" },
            { id = "submit", type = "submit", value = "Create" }
        }
    },
    {
        TYPE = "LINKS",
        LINKS = {{ value = "users", label = "View All Users" }}
    }
}
```

**Available view types:**
- `LINKS`: Navigation links
- `POSTFORM`/`PUTFORM`/`DELETEFORM`: Forms with different HTTP methods
- `MARKDOWN`: Render response data as markdown
- `OBJECT`: Display JSON objects in structured format
- `TABLE`: Tabular data display

## Advanced Patterns

### Multi-step forms with JWT state:
```lua
SETJWT = function(resp, jwt)
    if resp.step == "email_verified" then
        jwt = jwt or {}
        jwt.registration_step = "email_verified"
        jwt.temp_user_id = resp.temp_id
        return jwt
    end
    return jwt
end
```

### Role-based access control:
```lua
PREPROCESS = function(req, jwt)
    if not jwt or jwt.role ~= "admin" then
        error("Admin access required")
    end
    return req
end
```

### API with static frontend:
Place your SPA in `public/index.html` and create API routes under `api/` prefix for clean separation.

## Troubleshooting

**Common issues:**
1. **Parameter mismatch**: Request parameter names don't match SQL function parameters
2. **Missing migrations**: SQL functions reference tables that don't exist
3. **JWT issues**: SETJWT not returning proper table structure
4. **Route conflicts**: Static files conflicting with route definitions
5. **SQL syntax errors**: PostgreSQL function syntax issues

**Debugging steps:**
1. Check `config.lua` syntax and route structure
2. Verify SQL function parameter names match request parameters
3. Ensure migrations create necessary tables and columns
4. Test SQL functions independently in PostgreSQL
5. Validate JWT token structure and claims

This framework prioritizes simplicity and rapid development while maintaining the power and reliability of PostgreSQL as the core business logic layer.