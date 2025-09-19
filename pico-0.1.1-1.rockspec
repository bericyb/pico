package = 'pico'
version = '0.1.1-1'
source = {
  url = 'git+ssh://git@github.com/bericyb/pico.git',
}
description = {
  homepage = '*** please enter a project homepage ***',
  license = '*** please specify a license ***',
}
dependencies = {
  'lua >= 5.1, < 5.4',
  'lsqlite3complete',
  'lua-cjson',
  'inspect',
  'luafilesystem',
}
build = {
  type = 'builtin',
  modules = {
    main = 'main.lua',
  },
}
