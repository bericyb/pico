SPROC = {
  sql = "",
  parameters = {}
}

function SPROC:init(sql_file_name)
  local sql_file = io.open('sql/'..sql_file_name, 'r')
  if !sql_file then
    print("Unable to read " .. sql_file_name .. "\nIs your sql file in the sql/ directory?" )
    return self
  end
  ---@type string
  local function_def = sql_file:read('*l')

  local param_names = function_def:gmatch'@([^w]*)'

  local sql_str = sql_file:read("*a")

  for name in param_names do
    self.parameters[name] = ""
  end
end

function SPROC:run(parameters)
  self:sql
end

return SPROC
