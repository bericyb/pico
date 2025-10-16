Example = [[return {
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
    local chunk = dofile(name .. 'config.lua')
    for _, route in ipairs(chunk.ROUTES) do
      for _, method in ipairs(route) do
        if method.SQL then
          local function_name = method.SQL
          f = assert(io.open(name .. 'functions/' .. function_name .. '.sql', 'w'))

          f:write('CREATE OR REPLACE FUNCTION ' .. function_name .. '()\nRETURNS TABLE(example_result TEXT) AS $$\n\tSELECT * FROM table;\n$$ LANGUAGE sql;')
          f:close()
        end
      end
    end
  end
elseif flag == 'migrate' then
elseif flag == 'function' then
elseif flag == 'generate' or flag == 'ai' then
end
