CMD = arg[1]

if CMD == 'migrate' or CMD == 'm' then
  local ts = os.time()
  print 'enter migration name... '
  local name = io.read()

  name = string.gsub(name, '%s', '_')
  local filename = 'db/migrations/' .. ts .. '_' .. name .. '.lua'
  local file = assert(io.open(filename, 'w'))
  file:write 'M = {}\n\nfunction M:up()\n\treturn"Write your sql here"\nend\nreturn M'
end
