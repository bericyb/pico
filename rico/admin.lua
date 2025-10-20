local Example = [[return {
  DB = 'postgresql://postgres:password@0.0.0.0:5432/pico',
  ROUTES = {
    [''] = {
      GET = {
        VIEW = {
          {
            TYPE = 'LINKS',
            FIELDS = {
              { id = 'login', type = 'link', label = 'Login' },
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
              { id = 'username', type = 'text', label = 'Username' },
              { id = 'password', type = 'password', label = 'Password' },
              { id = 'button', type = 'submit', value = 'Login', label = 'Login' },
            },
          },
        },
      },
      POST = {
        SQL = 'login.sql',
        SETJWT = function(obj, jwt)
          return {
            userId = obj.id,
          }
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
        SQL = 'logout.sql',
        SETJWT = function()
          return nil
        end,
      },
    },
    ['user/:id'] = {
      GET = {
        SQL = 'getUser.sql',
        POLICY = function(obj, jwt)
          if obj.id == jwt.userId then
            return true
          else
            return false
          end
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
  os.execute('touch ' .. name .. 'config.lua')

  local f = assert(io.open(name .. 'config.lua', 'w'))
  f:write(Example)
  f:close()

  print 'Would you like to generate any migrations (m), functions (f), both (a) or not (n)?'
  local input = io.read '*l'
  if input == 'm' or input == 'a' then
    local time = os.time()
    local editor = 'vi'
    if assert(os.getenv 'EDITOR') then
      editor = assert(os.getenv 'EDITOR')
    end
    os.execute(editor .. ' ' .. name .. 'migrations/' .. time .. ':init.sql')
  end
  if input == 'f' or input == 'a' then
    local chunk = assert(dofile(name .. 'config.lua'))
    for route, method in pairs(chunk.ROUTES) do
      print('generating function for route ' .. route)
      for _method, handler in pairs(method) do
        if handler.SQL then
          local function_name = handler.SQL
          function_name = string.sub(function_name, 1, string.find(function_name, '[.]') - 1)
          print(function_name)
          f = assert(io.open(name .. 'functions/' .. function_name .. '.sql', 'w'))

          f:write('CREATE OR REPLACE FUNCTION ' .. function_name .. '()\nRETURNS TABLE(example_result TEXT) AS $$\n\tSELECT * FROM table;\n$$ LANGUAGE sql;')
          f:close()
        end
      end
    end
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
elseif flag == 'delete' or flag == 'd' then
end
