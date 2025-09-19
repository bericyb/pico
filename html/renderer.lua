local json = require 'cjson'
local template = require 'html.template'

M = {}

-- This function uses the structure of the route_view and the object returned from the DB
-- to create a fully rendered html page
function M:render_html(view_def, res_object)
  -- Collection of html strings.
  local entity_strings = {}

  for entity_type, entity in pairs(view_def) do
    if entity_type == 'LINKS' then
      table.insert(entity_strings, template.render_links(entity))
    end
    if entity_type == 'POSTFORM' then
      table.insert(entity_strings, template.render_postform(entity))
    end
    if entity_type == 'TABLE' then
      table.insert(entity_strings, template.render_table(res_object))
    end
    if entity_type == 'MARKDOWN' then
      table.insert(entity_strings, template.render_markdown(res_object))
    end
  end

  local html = template.body_open
  for _, entity_string in ipairs(entity_strings) do
    html = html .. entity_string
  end

  html = html .. template.body_close

  return html
end

return M
