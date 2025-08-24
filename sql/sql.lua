local sqlite = require 'lsqlite3'
SQL = {}

function SQL:init(db_connection)
  -- Set up DB connection
  -- For now we only support SQLite
  local db = sqlite.open(db_connection)
  self.DB = db

  -- Apply migrations
  -- Load "sprocs" into memory
  -- Parse out sproc input parameters into a table
end

-- In the future I'd like to be able to write in-line sql
-- instead of always having to use sql files.
function SQL:execute_sql(req_body, sql_file_name)
  local sproc = self.SPROCS[sql_file_name]

  local res, revert_func = sproc:run(self:DB, req_body)
  return res, revert_func
end

return SQL
