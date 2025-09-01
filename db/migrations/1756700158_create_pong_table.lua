M = {}

function M:up()
  return 'CREATE TABLE pong(pinged_at TEXT DEFAULT CURRENT_TIMESTAMP);'
end

return M

