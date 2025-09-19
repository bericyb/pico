M = {
  render_link = function(links)
    local sidebar = '<div>\n'
    for link in links do
      sidebar = sidebar .. '<a href="/' .. link .. '">\n<h3>' .. link .. '</h3>\n</a>\n'
    end
    sidebar = sidebar .. '</div>'

    return sidebar
  end,
}

return M
