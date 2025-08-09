local utils = require 'http.utils'
local inspect = require 'inspect'

PROCESSOR = {}

function PROCESSOR:init(config_file)
  ---@type function?, string?
  local config_chunk, err = loadfile(config_file)
  if not config_chunk then
    print('Config file: ' .. config_file .. ' was not not able to load properly exiting now.')
    print(err)
    return
  end

  -- This is naive and we'll need to change this later
  -- ie. converting routes into a data structure that
  -- allows for easier and faster pattern matching
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

  print(inspect(self.ROUTE_TREE))
end

function PROCESSOR:execute_request(request)
  -- Routes can be defined with route parameters signified
  -- by a colon. ie. user/:id/status where id is the name
  -- of the parameter. id will parsed into the body of
  -- the request.
  --
  -- At runtime, the request's path may be user/123/status
  -- and we must account for that.
  --
  -- Path will not contain query parameters since the http
  -- parser has moved them to the params field of request
  ---@type string
  local path = request.path
  local splits = path:gmatch '([^/]*)'
  local segments = {}
  for i in splits do
    print('path request segments', i)
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
      print(inspect(tree))
    elseif tree['*'] then
      print('found wildcard match for', seg)
      route_params[tree['*']['$name']] = seg
      tree = tree['*']
      print(inspect(tree))
    else
      print 'no match found returning NOT_FOUND'
      return utils.NOT_FOUND
    end
  end

  print('indentified route', tree['$route'])

  if not PROCESSOR.ROUTES[tree['$route']][request.method:upper()] then
    return utils.NOT_FOUND
  end

  for k, v in pairs(route_params) do
    request.body[k] = v
  end

  return 'FOUND'
end

return PROCESSOR
