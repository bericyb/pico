local Example = [[return {
  DB = 'postgresql://postgres:password@0.0.0.0:5432/pico',
  ROUTES = {
    [''] = {
      GET = {
        VIEW = {
          {
            TYPE = 'LINKS',
            LINKS = {
              { value = 'login', label = 'Login' },
              { value = 'register', label = 'Register' },
            },
          },
        },
      },
    },
    ['login'] = {
      GET = {
        VIEW = {
          {
            TYPE = 'POSTFORM',
            TITLE = 'Login',
            TARGET = '/login',
            FIELDS = {
              { id = 'user_email', type = 'email', label = 'Email' },
              { id = 'user_password', type = 'password', label = 'Password' },
              { id = 'button', type = 'submit', value = 'Login' },
            },
          },
        },
      },
      POST = {
        VIEW = {
          {
            TYPE = 'MARKDOWN',
          },
          {
            TYPE = 'LINKS',
            LINKS = {
              { value = '', label = 'Home' },
            },
          },
        },
        SQL = 'authenticate_user.sql',
        PREPROCESS = function(params, jwt)
          print('Login PREPROCESS:', params, 'JWT:', jwt)
          if jwt and jwt.userId then
            print('User already authenticated as:', jwt.userId)
          end
          return params
        end,
        POSTPROCESS = function(obj, jwt)
          print('Login POSTPROCESS:', obj, 'JWT:', jwt)
          if obj and obj.id then
            return 'Login successful! Welcome back.'
          else
            return 'Invalid email or password. Please try again.'
          end
        end,
        SETJWT = function(obj, jwt)
          if obj and obj.id then
            return {
              userId = obj.id,
              email = obj.email,
            }
          end
          return nil
        end,
      },
    },
    ['register'] = {
      GET = {
        VIEW = {
          {
            TYPE = 'POSTFORM',
            TITLE = 'Register',
            TARGET = '/register',
            FIELDS = {
              { id = 'user_email', type = 'email', label = 'Email' },
              { id = 'user_password', type = 'password', label = 'Password' },
              { id = 'button', type = 'submit', value = 'Register' },
            },
          },
        },
      },
      POST = {
        VIEW = {
          {
            TYPE = 'MARKDOWN',
          },
          {
            TYPE = 'LINKS',
            LINKS = {
              { value = 'login', label = 'Login' },
              { value = '', label = 'Home' },
            },
          },
        },
        SQL = 'register_user.sql',
        POSTPROCESS = function(obj, jwt)
          print('Registration POSTPROCESS:', obj, 'JWT:', jwt)
          if obj and obj.id then
            return 'Registration successful! Please login with your new account.'
          else
            return 'Registration failed. Email may already be in use.'
          end
        end,
      },
    },
    ['test-jwt'] = {
      GET = {
        PREPROCESS = function(params, jwt)
          print('Test PREPROCESS - params:', params, 'JWT:', jwt)
          if jwt then
            print('JWT found - userId:', jwt.userId)
            params.authenticated_user = jwt.userId
          else
            print 'No JWT found'
            params.authenticated_user = nil
          end
          return params
        end,
        POSTPROCESS = function(obj, jwt)
          print('Test POSTPROCESS - obj:', obj, 'JWT:', jwt)
          if jwt then
            return {
              message = 'Hello authenticated user ' .. (jwt.userId or 'unknown'),
              jwt_present = true,
              user_id = jwt.userId,
            }
          else
            return {
              message = 'Hello anonymous user',
              jwt_present = false,
            }
          end
        end,
      },
    },
    ['ping'] = {
      GET = {
        SQL = 'pong.sql',
      },
    },
    ['logout'] = {
      POST = {
        SETJWT = function()
          return nil
        end,
      },
    },
  },
}]]

local flag = arg[1]
if flag == 'init' then
  local name = ''
  if arg[2] then
    name = arg[2] .. '/'
  end
  os.execute('mkdir ' .. name)
  os.execute('mkdir ' .. name .. 'migrations/')
  os.execute('mkdir ' .. name .. 'functions/')
  os.execute('mkdir ' .. name .. 'public/')
  os.execute('touch ' .. name .. 'config.lua')

  local f = assert(io.open(name .. 'config.lua', 'w'))
  f:write(Example)
  f:close()

  -- Generate styles.css file
  local styles_content = [[/* Reset and base styles */
* {
    box-sizing: border-box;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
    line-height: 1.6;
    color: #333;
    background-color: #f8f9fa;
    margin: 0;
    padding: 20px;
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
}

/* Container for content */
body > * {
    width: 100%;
    max-width: 800px;
    margin-bottom: 2rem;
}

/* Typography */
h1, h2, h3, h4, h5, h6 {
    color: #2c3e50;
    margin: 0 0 1rem 0;
    font-weight: 600;
}

h1 { font-size: 2.5rem; }
h2 { font-size: 2rem; }
h3 { font-size: 1.5rem; }

p {
    color: #555;
    margin: 0 0 1rem 0;
}

/* Links */
a {
    color: #3498db;
    text-decoration: none;
    padding: 0.5rem 1rem;
    border-radius: 6px;
    display: inline-block;
    margin: 0.25rem;
    background-color: #fff;
    border: 1px solid #e1e8ed;
    transition: all 0.2s ease;
}

a:hover {
    background-color: #3498db;
    color: white;
    transform: translateY(-1px);
    box-shadow: 0 4px 8px rgba(52, 152, 219, 0.2);
}

/* Links container */
.links-container {
    display: flex;
    flex-direction: row;
    flex-wrap: wrap;
    gap: 0.5rem;
    background: white;
    padding: 1.5rem;
    border-radius: 12px;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
    border: 1px solid #e1e8ed;
    align-items: center;
}

.links-container a {
    margin: 0;
}

/* Forms */
form {
    background: white;
    padding: 2rem;
    border-radius: 12px;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
    border: 1px solid #e1e8ed;
    max-width: 400px !important;
}

legend {
    font-size: 1.25rem;
    font-weight: 600;
    color: #2c3e50;
    margin-bottom: 1.5rem;
    padding: 0;
    border: none;
}

label {
    display: block;
    margin-bottom: 0.5rem;
    font-weight: 500;
    color: #555;
}

input {
    width: 100%;
    padding: 0.75rem;
    border: 2px solid #e1e8ed;
    border-radius: 6px;
    font-size: 1rem;
    margin-bottom: 1rem;
    transition: border-color 0.2s ease;
    background-color: #fff;
}

input:focus {
    outline: none;
    border-color: #3498db;
    box-shadow: 0 0 0 3px rgba(52, 152, 219, 0.1);
}

input[type="submit"], button {
    background-color: #3498db;
    color: white;
    border: none;
    padding: 0.75rem 1.5rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 1rem;
    font-weight: 500;
    transition: background-color 0.2s ease;
    margin-top: 1rem;
}

input[type="submit"]:hover, button:hover {
    background-color: #2980b9;
    transform: translateY(-1px);
}

/* Tables */
table {
    width: 100%;
    border-collapse: collapse;
    background: white;
    border-radius: 12px;
    overflow: hidden;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
    border: 1px solid #e1e8ed;
}

th, td {
    padding: 1rem;
    text-align: left;
    border-bottom: 1px solid #e1e8ed;
}

th {
    background-color: #f8f9fa;
    font-weight: 600;
    color: #2c3e50;
    text-transform: uppercase;
    font-size: 0.875rem;
    letter-spacing: 0.05em;
}

tr:hover {
    background-color: #f8f9fa;
}

tr:last-child td {
    border-bottom: none;
}

/* Code/Pre elements */
pre {
    background: #2c3e50;
    color: #ecf0f1;
    padding: 1.5rem;
    border-radius: 8px;
    overflow-x: auto;
    font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
    font-size: 0.875rem;
    line-height: 1.5;
    border: 1px solid #34495e;
}

/* Markdown content */
md {
    display: block;
    background: white;
    padding: 2rem;
    border-radius: 12px;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
    border: 1px solid #e1e8ed;
}

/* Mobile responsive */
@media (max-width: 768px) {
    body {
        padding: 1rem;
    }
    
    h1 { font-size: 2rem; }
    h2 { font-size: 1.5rem; }
    h3 { font-size: 1.25rem; }
    
    form {
        padding: 1.5rem;
        max-width: 100% !important;
    }
    
    table {
        font-size: 0.875rem;
    }
    
    th, td {
        padding: 0.75rem 0.5rem;
    }
    
    pre {
        padding: 1rem;
        font-size: 0.8rem;
    }
    
    a {
        padding: 0.75rem;
        margin: 0.125rem;
    }
}

@media (max-width: 480px) {
    body {
        padding: 0.5rem;
    }
    
    form {
        padding: 1rem;
    }
    
    th, td {
        padding: 0.5rem 0.25rem;
    }
}

/* User-friendly data display */
.data-display {
    background: white;
    border-radius: 12px;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
    border: 1px solid #e1e8ed;
    overflow: hidden;
}

.data-cards {
    padding: 0;
}

.data-card {
    padding: 1.5rem 2rem;
    border-bottom: 1px solid #f0f0f0;
    display: flex;
    align-items: center;
    gap: 1.5rem;
    transition: background-color 0.2s ease;
}

.data-card:hover {
    background-color: #fafbfc;
}

.data-card:last-child {
    border-bottom: none;
}

.data-label {
    font-weight: 600;
    color: #2c3e50;
    min-width: 120px;
    flex-shrink: 0;
    text-transform: capitalize;
    font-size: 0.95rem;
}

.data-value {
    color: #555;
    font-size: 1rem;
    flex: 1;
    word-break: break-word;
    line-height: 1.5;
}

.empty-value {
    color: #95a5a6;
    font-style: italic;
    font-size: 0.9rem;
}

.error-message {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 2rem;
    background: linear-gradient(135deg, #fff5f5 0%, #fef5e7 100%);
    border-left: 4px solid #e74c3c;
}

.error-icon {
    font-size: 2rem;
    flex-shrink: 0;
}

.error-content h3 {
    color: #e74c3c;
    margin: 0 0 0.5rem 0;
    font-size: 1.1rem;
}

.error-content p {
    color: #c0392b;
    margin: 0;
    font-size: 0.9rem;
}

.technical-details {
    margin-top: 1rem;
    border-top: 1px solid #e1e8ed;
}

.technical-details summary {
    padding: 1rem 2rem;
    cursor: pointer;
    font-size: 0.9rem;
    color: #7f8c8d;
    background: #f8f9fa;
    border: none;
    outline: none;
    transition: background-color 0.2s ease;
}

.technical-details summary:hover {
    background: #e9ecef;
}

.technical-details[open] summary {
    background: #e9ecef;
    border-bottom: 1px solid #e1e8ed;
}

.json-fallback {
    background: #f8f9fa;
    color: #2c3e50;
    padding: 1.5rem 2rem;
    margin: 0;
    border: none;
    font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
    font-size: 0.8rem;
    line-height: 1.5;
    overflow-x: auto;
}

/* Enhanced data types styling */
.data-value[data-type="email"] a,
.data-value[data-type="url"] a,
.data-value[data-type="phone"] a {
    color: #3498db;
    text-decoration: none;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    background: rgba(52, 152, 219, 0.1);
    transition: all 0.2s ease;
}

.data-value[data-type="email"] a:hover,
.data-value[data-type="url"] a:hover,
.data-value[data-type="phone"] a:hover {
    background: rgba(52, 152, 219, 0.2);
    transform: translateY(-1px);
}

.data-value[data-type="date"] {
    color: #8e44ad;
    font-weight: 500;
}

.data-value[data-type="number"] {
    color: #e67e22;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
}

.data-value[data-type="boolean"] {
    font-weight: 500;
}

/* Text expansion */
.expand-text {
    background: none;
    border: none;
    color: #3498db;
    cursor: pointer;
    font-size: 0.85rem;
    margin-left: 0.5rem;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    transition: background-color 0.2s ease;
}

.expand-text:hover {
    background: rgba(52, 152, 219, 0.1);
}

.text-preview {
    display: inline;
}

.full-text {
    display: none;
    white-space: pre-wrap;
    word-break: break-word;
}

/* Status indicators */
.status-indicator {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.25rem 0.75rem;
    border-radius: 20px;
    font-size: 0.85rem;
    font-weight: 500;
}

.status-active {
    background: #d4edda;
    color: #155724;
}

.status-inactive {
    background: #f8d7da;
    color: #721c24;
}

.status-pending {
    background: #fff3cd;
    color: #856404;
}

/* Mobile responsive for data display */
@media (max-width: 768px) {
    .data-card {
        flex-direction: column;
        align-items: flex-start;
        gap: 0.5rem;
        padding: 1rem 1.5rem;
    }
    
    .data-label {
        min-width: auto;
        font-size: 0.9rem;
        color: #7f8c8d;
    }
    
    .data-value {
        font-size: 1rem;
        margin-top: 0.25rem;
    }
    
    .error-message {
        padding: 1.5rem;
        gap: 0.75rem;
    }
    
    .error-icon {
        font-size: 1.5rem;
    }
    
    .technical-details summary {
        padding: 0.75rem 1.5rem;
        font-size: 0.85rem;
    }
    
    .json-fallback {
        padding: 1rem 1.5rem;
        font-size: 0.75rem;
    }
}

@media (max-width: 480px) {
    .data-card {
        padding: 0.75rem 1rem;
    }
    
    .data-label {
        font-size: 0.85rem;
    }
    
    .data-value {
        font-size: 0.9rem;
    }
}]]

  local styles_file = assert(io.open(name .. 'public/styles.css', 'w'))
  styles_file:write(styles_content)
  styles_file:close()
  print('Created: ' .. name .. 'public/styles.css')

  print 'Would you like to generate any migrations (m), functions (f), both (a) or not (n)?'
  local input = io.read '*l'
  if input == 'm' or input == 'a' then
    local base_time = os.time()

    -- Migration 1: Enable pgcrypto extension
    local pgcrypto_content = [[-- Enable pgcrypto extension for password hashing
CREATE EXTENSION IF NOT EXISTS pgcrypto;]]

    local pgcrypto_file = assert(io.open(name .. 'migrations/' .. base_time .. ':enable_pgcrypto.sql', 'w'))
    pgcrypto_file:write(pgcrypto_content)
    pgcrypto_file:close()
    print('Created: ' .. name .. 'migrations/' .. base_time .. ':enable_pgcrypto.sql')

    -- Migration 2: Create users table
    local users_content = [[-- Users table for email/password authentication
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);]]

    local users_file = assert(io.open(name .. 'migrations/' .. (base_time + 1) .. ':create_users_table.sql', 'w'))
    users_file:write(users_content)
    users_file:close()
    print('Created: ' .. name .. 'migrations/' .. (base_time + 1) .. ':create_users_table.sql')

    -- Migration 3: Create ping counter table
    local ping_content = [[-- Ping counter table for tracking API usage
CREATE TABLE IF NOT EXISTS ping_counter (
    id SERIAL PRIMARY KEY,
    count INTEGER DEFAULT 0,
    last_ping TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);]]

    local ping_file = assert(io.open(name .. 'migrations/' .. (base_time + 2) .. ':create_ping_counter_table.sql', 'w'))
    ping_file:write(ping_content)
    ping_file:close()
    print('Created: ' .. name .. 'migrations/' .. (base_time + 2) .. ':create_ping_counter_table.sql')

    print 'Created 3 essential migrations for database setup'
  end
  if input == 'f' or input == 'a' then
    -- Create authentication functions
    local auth_user_content = [[CREATE OR REPLACE FUNCTION authenticate_user(user_email text, user_password text)
RETURNS TABLE(id int, email VARCHAR) AS $$
BEGIN
    RETURN QUERY
    SELECT users.id, users.email
    FROM users
    WHERE users.email = user_email 
    AND users.password_hash = crypt(user_password, users.password_hash);
END;
$$ LANGUAGE plpgsql;]]

    local register_user_content = [[CREATE OR REPLACE FUNCTION register_user(user_email text, user_password text)
RETURNS TABLE(id int, email VARCHAR) AS $$
BEGIN
    -- Check if user already exists
    IF EXISTS (SELECT 1 FROM users WHERE users.email = user_email) THEN
        RETURN; -- Return empty result if user exists
    END IF;
    
    -- Insert new user with hashed password
    RETURN QUERY
    INSERT INTO users (email, password_hash)
    VALUES (user_email, crypt(user_password, gen_salt('bf', 8)))
    RETURNING users.id, users.email;
END;
$$ LANGUAGE plpgsql;]]

    -- Create the authentication function files
    local auth_file = assert(io.open(name .. 'functions/authenticate_user.sql', 'w'))
    auth_file:write(auth_user_content)
    auth_file:close()
    print('Created: ' .. name .. 'functions/authenticate_user.sql')

    local register_file = assert(io.open(name .. 'functions/register_user.sql', 'w'))
    register_file:write(register_user_content)
    register_file:close()
    print('Created: ' .. name .. 'functions/register_user.sql')

    -- Create pong function for ping endpoint
    local pong_content = [[CREATE OR REPLACE FUNCTION pong()
RETURNS TABLE(message text, count int, last_ping timestamp) AS $$
BEGIN
    -- Insert or update ping counter
    INSERT INTO ping_counter (id, count, last_ping)
    VALUES (1, 1, CURRENT_TIMESTAMP)
    ON CONFLICT (id)
    DO UPDATE SET 
        count = ping_counter.count + 1,
        last_ping = CURRENT_TIMESTAMP;
    
    -- Return the current state
    RETURN QUERY
    SELECT 
        'pong'::text as message,
        ping_counter.count,
        ping_counter.last_ping
    FROM ping_counter
    WHERE ping_counter.id = 1;
END;
$$ LANGUAGE plpgsql;]]

    local pong_file = assert(io.open(name .. 'functions/pong.sql', 'w'))
    pong_file:write(pong_content)
    pong_file:close()
    print('Created: ' .. name .. 'functions/pong.sql')
  end
elseif flag == 'migrate' or flag == 'm' then
  io.write 'Migration name: '
  local input = io.read '*l'

  if input == nil then
    print 'Error: Could not read migration name.'
    return
  end

  input = string.gsub(input, ' ', '_')
  input = string.gsub(input, '^%s*(.-)%s*$', '%1')

  if input == '' then
    print 'Migration name required'
    return
  end

  local now = os.time()

  local file_name = string.format('migrations/%d:%s.sql', now, input)

  local file, err = io.open(file_name, 'w')

  if file then
    file:close()
    print(string.format('Migration file %s created.', file_name))
  else
    print(string.format('migration creation failed: %s', err))
  end
elseif flag == 'function' or flag == 'f' then
  local SQL_FUNCTION_TEMPLATE = [[
  CREATE OR REPLACE FUNCTION %s(example_parameter int)
  RETURNS TABLE(example_result text) AS $$
	  <SQL STATEMENTS>;
  $$ LANGUAGE sql;
  ]]
  io.write 'SQL function name: '
  local input = io.read '*l'

  if input == nil then
    print 'Error reading input.'
    return
  end

  input = string.gsub(input, ' ', '_')
  input = string.gsub(input, '^%s*(.-)%s*$', '%1')

  if input == '' then
    print 'Function name required'
    return
  end

  local file_path = string.format('functions/%s.sql', input)

  local exists_check = io.open(file_path, 'r')
  if exists_check then
    exists_check:close()
    print(string.format("function creation failed: File '%s' already exists.", file_path))
    return
  end

  local file, err = io.open(file_path, 'w')

  if not file then
    print(string.format('function creation failed: %s', err))
    return
  end

  local content = string.gsub(SQL_FUNCTION_TEMPLATE, '{name}', input)

  local success, write_err = file:write(content)

  if not success then
    print(string.format('function creation failed: Failed to write content: %s', write_err))
    file:close()
    return
  end

  -- Close the file handle
  file:close()

  print(string.format('Function file %s created.', input))
elseif flag == 'generate' or flag == 'ai' then
else
  print 'Usage: lua admin.lua [init|migrate|function|generate|delete]'
end
