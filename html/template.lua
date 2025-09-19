local inspect = require 'inspect'

M = {
  body_open = [[<!DOCTYPE html>
<html lang="en">
<head>
	<meta charset="UTF-8">
	<meta name="viewport" content="width=device-width, initial-scale=1.0">
	<title>Pico</title>
</head>
<body>]],
  body_close = '</body>\n</html>',
  render_links = function(links)
    local link_entities = {}
    for _, link in ipairs(links) do
      table.insert(link_entities, string.format('<a href="/%s">%s</a>', link, Capitalize(link)))
    end
    return table.concat(link_entities, '\n')
  end,
  render_postform = function(entity)
    local form = { '<form method="post">' }
    if entity.TITLE then
      table.insert(form, string.format('<h3>%s</h3>', Capitalize(entity.TITLE)))
    end
    if entity.FIELDS then
      for _, field in pairs(entity.FIELDS) do
        table.insert(form, create_field(field))
      end
    end
    table.insert(form, '</form>')
    return table.concat(form, '\n')
  end,
  render_table = function(objects)
    return 'please implement table renderer'
  end,
  render_markdown = function(text)
    return 'please implement markdown renderer'
  end,
}

-- Fields are inputs defined as a table
-- {
--	name: the url encoded parameter name
--	type: the input type ie. text, textarea, checkbox, or other html input types
--	label: end user facing label present in the html
-- }
function create_field(field)
  local input = ''
  if field.label then
    input = string.format('<label for="%s">%s\n</label>', field.name, field.label)
  end
  input = input .. '<input '
  for attr, val in pairs(field) do
    if attr ~= 'label' then
      input = input .. string.format('%s="%s"', attr, val)
    end
  end
  return input .. ' />'
end

function create_button(def)
  local button = '<button '
  button = button .. string.format('type="%s"', def.type)
  button = button .. string.format('name="%s"', def.name)
  button = button .. string.format('> %s</button', def.label)
  return button
end

function Capitalize(str)
  return str:gsub('^%l', string.upper)
end

return M
