local cjson = require 'cjson'

M = {
  BAD_REQUEST = 'HTTP/1.1 400 Bad Request\r\n\r\n',
  INTERNAL_ERROR = 'HTTP/1.1 500 Internal Error\r\n\r\n',
  UNAUTHORIZED = 'HTTP/1.1 403 Unauthorized\r\n\r\n',
  NOT_FOUND = 'HTTP/1.1 404 Not Found\r\n\r\n',
  OK = 'HTTP/1.1 200 OK\r\n',
}

-- These functions parse out the raw body into the request body
Content_parsers = {
  ['application/json'] = function(r)
    if r.raw_body == nil then
      r.body = {}
    else
      r.body = cjson.decode(r.raw_body)
    end
  end,
  ['application/x-www-form-urlencoded'] = function(r)
    for key, val in pairs(r.params) do
      r.body[key] = val
    end
  end,
  -- RFC says that most of the time the boundary string is surrounded by quotes
  -- but curl doesn't do that I guess?
  ['multipart/form-data'] = function(r)
    local boundary = r.body:match '[^=]*=(.*)'
    if boundary[0] == '"' then
      boundary = boundary:match '[^"]*(.*)[^"]'
    end
    r.body['todo'] = 'I still gotta implement multipart forms 0.0'
  end,
  ['text/html'] = function(r)
    r.body = { r.raw_body }
  end,
}

function M.parse_content(request)
  local content_parser = Content_parsers[request.headers['content-type']]
  if content_parser ~= nil then
    content_parser(request)
  end
end

return M
