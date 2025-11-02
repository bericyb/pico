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
  f:write(CONFIG_TEMPLATE)
  f:close()

  -- Generate styles.css file
  local styles_file = assert(io.open(name .. 'public/styles.css', 'w'))
  styles_file:write(STYLES_TEMPLATE)
  styles_file:close()
  print('Created: ' .. name .. 'public/styles.css')

  -- Generate AGENTS.md file
  local agents_file = assert(io.open(name .. 'AGENTS.md', 'w'))
  agents_file:write(AGENTS_TEMPLATE)
  agents_file:close()
  print('Created: ' .. name .. 'AGENTS.md')

  print 'Would you like to generate any migrations (m), functions (f), both (a) or not (n)?'
  local input = io.read '*l'
  if input == 'm' or input == 'a' then
    local base_time = os.time()

    -- Migration 1: Enable pgcrypto extension
    local pgcrypto_file = assert(io.open(name .. 'migrations/' .. base_time .. ':enable_pgcrypto.sql', 'w'))
    pgcrypto_file:write(MIGRATION_PGCRYPTO_TEMPLATE)
    pgcrypto_file:close()
    print('Created: ' .. name .. 'migrations/' .. base_time .. ':enable_pgcrypto.sql')

    -- Migration 2: Create users table
    local users_file = assert(io.open(name .. 'migrations/' .. (base_time + 1) .. ':create_users_table.sql', 'w'))
    users_file:write(MIGRATION_USERS_TABLE_TEMPLATE)
    users_file:close()
    print('Created: ' .. name .. 'migrations/' .. (base_time + 1) .. ':create_users_table.sql')

    -- Migration 3: Create ping counter table
    local ping_file = assert(io.open(name .. 'migrations/' .. (base_time + 2) .. ':create_ping_counter_table.sql', 'w'))
    ping_file:write(MIGRATION_PING_COUNTER_TEMPLATE)
    ping_file:close()
    print('Created: ' .. name .. 'migrations/' .. (base_time + 2) .. ':create_ping_counter_table.sql')

    print 'Created 3 essential migrations for database setup'
  end
  if input == 'f' or input == 'a' then
    -- Create authentication functions
    -- Create the authentication function files
    local auth_file = assert(io.open(name .. 'functions/authenticate_user.sql', 'w'))
    auth_file:write(FUNCTION_AUTHENTICATE_USER_TEMPLATE)
    auth_file:close()
    print('Created: ' .. name .. 'functions/authenticate_user.sql')

    local register_file = assert(io.open(name .. 'functions/register_user.sql', 'w'))
    register_file:write(FUNCTION_REGISTER_USER_TEMPLATE)
    register_file:close()
    print('Created: ' .. name .. 'functions/register_user.sql')

    -- Create pong function for ping endpoint
    local pong_file = assert(io.open(name .. 'functions/pong.sql', 'w'))
    pong_file:write(FUNCTION_PONG_TEMPLATE)
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

  local content = string.gsub(FUNCTION_TEMPLATE, '{name}', input)

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
