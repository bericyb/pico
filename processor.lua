local utils = require 'http.utils'
local inspect = require 'inspect'
local SQL = require 'sql.sql'

PROCESSOR = {}

function PROCESSOR:init(config_file)
  ---@type function?, string?
  local config_chunk, err = loadfile(config_file)
  if not config_chunk then
    print('Config file: ' .. config_file .. ' was not not able to load properly exiting now.')
    print(err)
    return
  end

  local config = config_chunk()
  for k, v in pairs(config) do
    print('adding key: ', k, 'value: ', inspect(v))
    self[k] = v
  end

  -- Parse out route definitions into a nested table or tree
  self.ROUTE_TREE = {}

  for k, _ in pairs(self.ROUTES) do
    local segments = k:gmatch '([^/]*)'
    local current_branch
    for seg in segments do
      if seg ~= '' then
        local seg_name = seg
        print('segment here', seg)
        if string.find(seg, '^:') then
          seg_name = seg:match '^:(.*)'
          seg = '*'
        end
        if not current_branch then
          self.ROUTE_TREE[seg] = { ['$name'] = seg_name, ['$route'] = k }
          current_branch = self.ROUTE_TREE[seg]
        else
          current_branch[seg] = { ['$name'] = seg_name, ['$route'] = k }
          current_branch = current_branch[seg]
        end
      end
    end
  end

  self.SQL = SQL:init(self.DB)
  print(inspect(self))
  print(inspect(self.SQL))
  return self
end

function PROCESSOR:process_request(request)
  -- Routes can be defined with route parameters signified
  -- by a colon. ie. user/:id/status where id is the name
  -- of the parameter. id will parsed into the body of
  -- the request.
  --
  -- At runtime, the request's path may be user/123/status
  -- and we must account for that with a '*' entry in the table
  --
  -- Path will not contain query parameters since the http
  -- parser has moved them to the params field of request
  ---@type string
  local path = request.path
  local splits = path:gmatch '([^/]*)'
  local segments = {}
  for i in splits do
    if i ~= '' then
      table.insert(segments, i)
    end
  end

  local route_params = {}
  local tree = PROCESSOR.ROUTE_TREE
  for _, seg in ipairs(segments) do
    if tree[seg] then
      print('found segment match', seg)
      tree = tree[seg]
    elseif tree['*'] then
      print('found wildcard match for', seg)
      route_params[tree['*']['$name']] = seg
      tree = tree['*']
    else
      return nil, utils.NOT_FOUND
    end
  end

  local route = tree['$route']

  local route_definition = PROCESSOR.ROUTES[route][request.method:upper()]
  if route == nil or not PROCESSOR.ROUTES[route][request.method:upper()] then
    -- Check for a view method if a Get doesn't exist
    if request.method:upper() == 'GET' and PROCESSOR.ROUTES[route]['VIEW'] then
      route_definition = PROCESSOR.ROUTES[route]['VIEW']
    else
      print 'route method not found'
      return nil, utils.NOT_FOUND
    end
  end

  -- Adding parsed route parameters to body
  for k, v in pairs(route_params) do
    request.body[k] = v
  end

  -- 1. Execute SQL
  local sql_file_name = route_definition.SQL
  local res, commit, revert = self.SQL:execute_sql(request.body, sql_file_name)

  local jwt = request.headers['authorization']

  -- 2. Run policy
  if route_definition.POLICY then
    if not route_definition.POLICY(res, jwt) then
      revert()
      return nil, utils.UNAUTHORIZED
    end
  end
  commit()

  -- 3. SetJWT
  if route_definition.SETJWT then
    jwt = route_definition.SETJWT(res, jwt)
  end

  return res, nil
end

return PROCESSOR
