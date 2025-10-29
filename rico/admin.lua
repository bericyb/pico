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
    local chunk = assert(dofile(name .. 'config.lua'))

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
