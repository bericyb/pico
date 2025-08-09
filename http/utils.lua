local cjson = require 'cjson'

M = {
  BAD_REQUEST = 'HTTP/1.1 400 Bad Request\r\n\r\n',
  INTERNAL_ERROR = 'HTTP/1.1 500 Internal Error\r\n\r\n',
}

function M.parse_content(request)
  Content_parsers[request.headers['content-type']]()
end

-- These functions parse out the raw body into the request body
Content_parsers = {
  ['application/json'] = function(r)
    r.body = cjson.parse(r.raw_body)
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
}

return M
