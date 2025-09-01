local sqlite = require 'lsqlite3complete'
local fs = require 'lfs'
local inspect = require 'inspect'
SQL = {
  SPROCS = {},
}

function SQL:init(db_connection)
  -- Set up DB connection
  -- For now we only support SQLite
  local db = sqlite.open(db_connection)
  self.DB = db

  -- Apply migrations
  local pico_is_ready = false
  for _ in db:urows "SELECT name from sqlite_master WHERE type='table' and name='pico_migrations'" do
    pico_is_ready = true
  end

  -- Bootstrap pico migration system
  if not pico_is_ready then
    print 'bootstrapping pico...'
    db:exec 'CREATE TABLE pico_migrations(id)'
  end

  local latest_migration = 0
  for last_id in db:urows 'SELECT id FROM pico_migrations ORDER BY rowid DESC LIMIT 1' do
    latest_migration = last_id
  end
  local path = 'db/migrations'
  for file in fs.dir(path) do
    if file ~= '.' and file ~= '..' then
      local f = path .. '/' .. file
      local attr = fs.attributes(f)
      assert(type(attr) == 'table')
      if attr.mode ~= 'directory' then
        local ts = tonumber(string.match(file, '^[^_]+'))
        if ts > latest_migration then
          local migrator = dofile(f)
          local up_sql = migrator:up()
          print('applying migration ' .. f)
          db:exec(up_sql)
          db:exec('INSERT INTO pico_migrations values(' .. tostring(ts) .. ')')
        end
      end
    end
  end

  -- Load "sprocs" into memory
  path = 'db/sprocs'
  for file in fs.dir(path) do
    if file ~= '.' and file ~= '..' then
      local f = path .. '/' .. file
      print('\t ' .. f)
      local attr = fs.attributes(f)
      assert(type(attr) == 'table')
      if attr.mode ~= 'directory' then
        local sql_file = assert(io.open(f, 'r'))
        local sql = sql_file:read '*a'
        if sqlite.complete(sql) then
          self.SPROCS[file] = sql
        else
          error('sql file ' .. f .. ' does not contain one of more complete sql statements', 2)
        end
      end
    end
  end

  -- Parse out sproc input parameters into a table
  -- TODO:
  print('self db init', inspect(self.DB))
  return self
end

-- In the future I'd like to be able to write in-line sql
-- instead of always having to use sql files.
function SQL:execute_sql(req_body, sql_file_name)
  local sql = self.SPROCS[sql_file_name]

  self.DB:exec 'BEGIN'
  local res = {}

  local ok = pcall(function()
    self.DB:exec(sql, function(_, ncols, vals, names)
      local row = {}
      for i = 1, ncols do
        row[names[i]] = vals[i]
      end

      res = row
      return 0
    end)
  end)

  local revert_func = function()
    self.DB:exec 'ROLLBACK'
  end

  local commit_func = function()
    self.DB:exec 'COMMIT'
  end

  if not ok then
    revert_func()
  end

  return res, commit_func, revert_func
end

return SQL
