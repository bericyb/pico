local utils = require 'http.utils'
local inspect = require 'inspect'
local SQL = require 'sql.sql'
local html = require 'html.renderer'

PROCESSOR = {}

function PROCESSOR:init(config_file)
  ---@type function?, string?
  local config_chunk, err = loadfile(config_file)
  if not config_chunk then
    print('Config file: ' .. config_file .. ' was not not able to load properly exiting now.')
    error(err)
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
  print('full path', path)
  local splits = path:gmatch '([^/]+)'
  local segments = {}
  for seg in splits do
    table.insert(segments, seg)
  end

  local route_params = {}
  local tree = PROCESSOR.ROUTE_TREE
  if #segments == 0 and tree[''] then
    tree = tree['']
  else
    for _, seg in ipairs(segments) do
      if seg ~= '' then
        if tree[seg] then
          print('found segment match', seg)
          tree = tree[seg]
        elseif tree['*'] then
          print('found wildcard match for', seg)
          route_params[tree['*']['$name']] = seg
          tree = tree['*']
        else
          print '404 route not found'
          return nil, nil, utils.NOT_FOUND
        end
      else
        break
      end
    end
  end

  local route = tree['$route']

  local route_definition = PROCESSOR.ROUTES[route][request.method:upper()]
  if route == nil or not PROCESSOR.ROUTES[route][request.method:upper()] then
    print 'route method not found'
    return nil, nil, utils.NOT_FOUND
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
      return nil, nil, utils.UNAUTHORIZED
    end
  end
  commit()

  -- 3. SetJWT
  if route_definition.SETJWT then
    -- Gotta do a bit more here...
    jwt = route_definition.SETJWT(res, jwt)
  end

  -- 4. If the client prefers text/html and View is set, render out the page
  local accept = 'application/json'
  local accepts = request.headers['accept']:gmatch '([^,]*)'
  for accept_type in accepts do
    print('accept field', accept_type)
    if accept_type == 'text/html' or accept_type == 'application/json' then
      accept = accept_type
      break
    end
  end
  if accept == 'text/html' and route_definition.VIEW then
    res = html:render_html(route_definition.VIEW, res)
  elseif route_definition.VIEW == nil then
    accept = 'application/json'
  end

  local response = {
    body = res,
    content_type = accept,
  }

  print('Heres the response', inspect(response))

  return response, jwt, nil
end

return PROCESSOR
