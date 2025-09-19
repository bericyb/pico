local utils = require 'http.utils'
local cjson = require 'cjson'

HTTP = {}

function HTTP:parse_stream(stream)
  ---@type string, string | nil
  local line, err = stream:receive '*l'
  if err then
    stream:send(utils.BAD_REQUEST)
    return nil
  end

  if line then
    ---@type string, string
    local method, full_path = line:match '^(%w+) (.-) HTTP/1%.1'
    if not method or not full_path then
      stream:send(utils.BAD_REQUEST)
      return nil
    else
      -- Form the request
      local req = {
        method = method,
        path = full_path,
        params = {},
        headers = {},
      }

      -- Check for query parameters
      ---@type string, string
      local path, qps = full_path:match '([^?]*)?(.*)'
      if path and qps then
        req.path = path

        -- parse them out
        for key, val in qps:gmatch '([^&=?]+)=([^&=?]+)' do
          if key and val then
            req.params[key] = val
          end
        end
      end

      -- Parse headers
      while true do
        line = stream:receive '*l'
        local key, value = line:match '^(.-):%s*(.-)$'
        if key and value then
          req.headers[key:lower()] = value
        else
          break
        end
      end

      -- Read body
      local body_len = tonumber(req.headers['content-length'])
      if body_len and body_len > 0 then
        req.raw_body = stream:receive(body_len)
      end

      utils.parse_content(req)

      return req
    end
  else
    stream:send(utils.BAD_REQUEST)
    return {}
  end
end

function HTTP:build_response(req, response, jwt)
  local resp = utils.OK .. 'Server: Pico\r\n'
  if jwt then
    resp = resp .. string.format('Authorization: Bearer %s\r\n', jwt)
  end
  if response.content_type == 'text/html' or response.content_type == 'Text/HTML' then
    local len = #response.body
    resp = resp .. 'Content-Type: text/html\r\nContent-Length: ' .. len .. '\r\n\r\n' .. response.body
  else
    local encoded = cjson.encode(response.body)
    local len = #encoded
    resp = resp .. 'Content-Type: application/json\r\nContent-Length: ' .. len .. '\r\n\r\n' .. encoded
  end

  return resp
end

return HTTP
