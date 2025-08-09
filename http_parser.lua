local utils = require 'http.utils'

HTTP_PARSER = {}

function HTTP_PARSER:parse_stream(stream)
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

      -- Handle the fully parsed out request :)
      ---@type string
      local response = self.processor:process_request(req)

      stream:send(response)
    end
  else
    stream:send(utils.BAD_REQUEST)
    return {}
  end
end

return HTTP_PARSER
