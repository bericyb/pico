local socket = require 'socket'
local http = require 'http.http'
local processor = require 'processor'

print 'Starting PICO server...'
print 'Loading server configuration...'
local config_file_path = 'config.lua'
if arg[1] then
  print('Configuration argument was given, using ' .. arg[1] .. ' as root configuration file')
  config_file_path = arg[1]
else
  print 'No configuration argument was given, defaulting to ./config.lua...'
end

Proc = processor:init(config_file_path)

local server = socket.bind('::', 3000)
if server == nil then
  error 'error binding on ::3000'
end
while 1 do
  local stream = server:accept()
  stream:settimeout(10) -- Set a timeout of 10 seconds for client operations

  local req = http:parse_stream(stream)
  if not req then
    stream:close()
  else
    local resp, jwt, err = Proc:process_request(req)
    if err ~= nil then
      stream:send(err)
    else
      local http_resp = http:build_response(req, resp, jwt)
      stream:send(http_resp)
    end
  end

  stream:close()
end
