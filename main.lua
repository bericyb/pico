local socket = require 'socket'
local http_parser = require 'http_parser'
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

processor:init(config_file_path)

local server = socket.bind('::', 12345)
while 1 do
  local stream = server:accept()
  stream:settimeout(10) -- Set a timeout of 10 seconds for client operations

  local req = http_parser:parse_stream(stream)
  if not req then
    stream:close()
  else
    local resp = processor:execute_request(req)
    stream:send(resp)
  end

  stream:close()
end
