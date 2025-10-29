# What is Pico?

Pico is a minimalistic framework that defines apps using SQL and Lua. With ~80 lines of Lua and a few SQL functions, you can define a fullstack web application, For example, a realtime chat room application with authentication, frontend, and persistent data.

# What is Pico for?

Pico is not 1.0 yet, and should not be used for production workloads. However, Pico's philosophy is that relational SQL is a resilliant and healthy foundation for any CRUD application. Pico is small, and fast (in terms of developer experience), which lends itself to quick prototyping and delivery of value. 

Whenever you find yourself saying:
    "I just want to store this in a database, and see it on a webpage",
    "I just want a simple HTTP API for this data",
    "I just want to write some raw SQL, I don't need a damn ORM."
Pico is a good option.

If you reach a point where Pico isn't right for you, it's easy to leave. Pico is built upon Postgres which provides plenty of escape hatches and scalability. Leaving Pico is easy as running pg_dump or making a new connection to the Postgres database from any other application layer.

# Getting Started

To create a new Pico application run:
```shell
pico init <application_name> # New directory with a Pico app
# --- or ---
pico init                    # Initialize current directory as a Pico app

```

Pico apps have the following structure
```
Application
├── config.lua
├── functions
│   └── pong.sql
└── migrations
    └── 1760832777:init.sql
```

`config.lua` is where you define your application's database connection and endpoints/routes as a Lua table.
```lua 
return {
    DB = 'connection_string',
    ROUTES = { ... }
}
```
## DB
DB is just a connection string to your Postgres database

## ROUTES
Routes is a definition of your endpoints, 
They're defined with the url route they're available at, their accepted methods, and respective handlers which is zero or more of (PREPROCESS, SQL, POSTPROCESS, SETJWT, VIEW) executed in that order.

Here's a simple ping route that utilizes all handlers that you'll find in the default configuration.

```lua
ROUTES = {
	['ping'] = {
		GET = {
			PREPROCESS = function(req)
				print("user has ping'd the server!")
				return req
			end,
			SQL = "get_num_pings.sql", -- SELECT COUNT(1) FROM pings;
			POSTPROCESS = function(resp)
				return 'There has been' .. resp .. 'pings'
			end,
			SETJWT = function(resp, jwt)
				if jwt == nil then
					resp = resp + "\n and you are unauthenticated"	
				else
					resp = "Welcome back " .. jwt.user_name .. "\n" .. resp
					return jwt
				end
			end,
			VIEW = {
				{ TYPE = "MARKDOWN" }
			},
		}
	}
}

```

| Handlers    | Usage                                                                                                                                                                                                                                     |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| PREPROCESS  | A Lua function whose input is the request's body and returns a new request body.  Used to pre-process a request's body in preparation for SQL execution. Helpful for validation, data manipulation, etc before SQL.                       |
| SQL         | The name of a SQL file containing the Function you want to execute on request to this route.                                                                                                                                              |
| POSTPROCESS | A Lua function whose input is the response from the SQL handler and returns a new response body. Helpful for executing logic on SQL responses and transforming SQL responses.                                                             |
| SETJWT      | A Lua function whose input is the current response body and the current JWT claims and returns a table to be used as the new JWT. Helpful for using SQL results to authenticate users, add and take away permissions or persist sessions. |
| VIEW        | A table of entities used to render an HTML response. Used to build a rudimentary frontend. More on views [here](VIEWS.md)                                                                                                                 |


## Request Formation and Parameter Mapping

**Critical Requirement**: For SQL functions to work properly, you **MUST** match parameter names in the request body to the parameter names in the SQL function.

### How Pico Processes Requests

Pico "squashes" all request data into a single body structure, regardless of whether the original request contains:
- JSON body data
- Query parameters (`?name=value&email=test@example.com`)
- Form data
- URL path parameters (`:id` in routes)

All of this data becomes available as key-value pairs that are mapped to your SQL function parameters.

### Parameter Mapping Rules

1. **Exact Name Matching**: The key names in your request body must exactly match the parameter names in your SQL function
2. **Case Sensitive**: Parameter names are case-sensitive
3. **Priority Order**: 
   - First, Pico looks in the request body (JSON or form data)
   - Then, it falls back to route parameters (like `:id` from URL paths)
   - If a required parameter is missing, the request fails with a BadRequest error

### Examples

#### SQL Function Definition
```sql
-- functions/create_user.sql
CREATE OR REPLACE FUNCTION create_user(username text, email text, age int)
RETURNS TABLE(id int, created_at timestamp) AS $$
    INSERT INTO users (username, email, age, created_at)
    VALUES (create_user.username, create_user.email, create_user.age, NOW())
    RETURNING id, created_at;
$$ LANGUAGE sql;
```

#### Valid Request Bodies

**JSON Request:**
```json
{
  "username": "john_doe",
  "email": "john@example.com", 
  "age": 25
}
```

**Form Data Request:**
```
POST /users
Content-Type: application/x-www-form-urlencoded

username=john_doe&email=john@example.com&age=25
```

**Query Parameters (GET request):**
```
GET /users?username=john_doe&email=john@example.com&age=25
```

#### Invalid Request Example
```json
{
  "name": "john_doe",     // ❌ Wrong! Should be "username"
  "userEmail": "john@example.com",  // ❌ Wrong! Should be "email"
  "age": 25               // ✅ Correct
}
```

### Route Parameters

URL parameters are also mapped to SQL function parameters:

```lua
-- Route definition
ROUTES = {
    ['users/:user_id'] = {
        GET = {
            SQL = "get_user_by_id.sql"  -- Function expects parameter named "user_id"
        }
    }
}
```

```sql
-- functions/get_user_by_id.sql
CREATE OR REPLACE FUNCTION get_user_by_id(user_id int)
RETURNS TABLE(id int, username text, email text) AS $$
    SELECT u.id, u.username, u.email 
    FROM users u 
    WHERE u.id = get_user_by_id.user_id;
$$ LANGUAGE sql;
```

A request to `GET /users/123` will automatically pass `user_id = 123` to the SQL function.

## Advanced Configuration

Because everything is a Lua table, you can decompose your `config.lua` into different files for simplicity.
For example:
```lua
--- config.lua
return {
    ...
    ROUTES = {
        ['login/'] = require('login_handler')
    }
    ...
} 

--- users_handlers.lua
return {
	POST = {
		PREPROCESS = function(request_body)
			-- logic
			request_body.name = "name_override"
			return request_body
		end,
		SQL = get_user.sql
		POSTPROCESS = function(sql_obj)
			if sql_obj.id == nil then
				return "No user found..."
			end
			return sql_obj
		end,
		SETJWT = function (resp_body, jwt)
			if resp_body.id then
				jwt.user_id = resp_body.id
				resp = "Login Successful"
			else 
				jwt = {}
			end
		end,
		VIEW = {
			TYPE = "MARKDOWN"
		},
	}
}

```



