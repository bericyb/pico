local flag = arg[1]

if flag == 'init' then
  local name = ''
  if arg[2] then
    name = arg[2] .. '/'
  end
  os.execute('mkdir ' .. name)
  os.execute('mkdir ' .. name .. 'migrations/')
  os.execute('mkdir ' .. name .. 'functions/')
  os.execute('touch ' .. name .. 'config.lua')

  local f = assert(io.open(name .. 'config.lua'))
  f.write(require 'example')
elseif flag == 'migrate' then
elseif flag == 'function' then
elseif flag == 'generate' then
end
