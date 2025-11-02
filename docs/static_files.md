# Static File Serving

Pico automatically serves static files from your `public/` directory when no matching route is found. This lets you serve CSS, JavaScript, images, and other assets alongside your dynamic routes without any extra configuration.

## What are Static Files for?

Static file serving is perfect when you need to:
- Serve CSS stylesheets and JavaScript files
- Host images, fonts, and other media assets
- Provide downloadable files like PDFs or documents
- Serve a simple HTML frontend alongside your API
- Host assets for single-page applications

## How It Works

1. **Routes First**: Pico checks your `config.lua` routes first
2. **Static Fallback**: If no route matches, Pico looks in the `public/` directory
3. **File Found**: Serves the file with proper MIME type and headers
4. **No File**: Returns 404 if nothing exists

```
Application/
├── config.lua
├── functions/
└── public/              # Put your static files here
    ├── index.html       # Served at /
    ├── styles.css       # Served at /styles.css
    ├── app.js          # Served at /app.js
    └── images/
        └── logo.png     # Served at /images/logo.png
```

## File Types

Pico automatically detects the right MIME type based on file extensions:

```
.html    → text/html
.css     → text/css
.js      → application/javascript
.png     → image/png
.jpg     → image/jpeg
.json    → application/json
.pdf     → application/octet-stream
```

## Files with Spaces and Special Characters

Pico properly handles files with spaces in their names through URL encoding:

```bash
# File: public/My Document - Final Version.pdf
# Access via: /My%20Document%20-%20Final%20Version.pdf

# File: public/Tatsuro Yamashita - Come Along.mp3  
# Access via: /Tatsuro%20Yamashita%20-%20Come%20Along.mp3
```

The `%20` represents URL-encoded spaces. Pico automatically decodes these to find your actual files.

## Directory Indexing

When you request a directory path ending with `/`, Pico automatically looks for `index.html`:

```
public/
├── index.html          # Served at /
└── docs/
    └── index.html      # Served at /docs/
```

## Security

Pico includes protection against common attacks:

**Path Traversal Blocked:**
```bash
GET /../etc/passwd           # ❌ Blocked
GET /images/../../../secrets # ❌ Blocked  
GET /test%2F..%2Fetc        # ❌ Blocked (URL-encoded)
```

Only files inside `public/` can be served - nothing else on your server is accessible.

## Simple Website Example

Create a basic website structure:

```html
<!-- public/index.html -->
<!DOCTYPE html>
<html>
<head>
    <title>My Pico App</title>
    <link rel="stylesheet" href="/styles.css">
</head>
<body>
    <h1>Welcome to Pico!</h1>
    <p>This is served from public/index.html</p>
    <button onclick="testAPI()">Test API</button>
    <script src="/app.js"></script>
</body>
</html>
```

```css
/* public/styles.css */
body {
    font-family: Arial, sans-serif;
    padding: 20px;
    background: #f5f5f5;
}
```

```javascript
// public/app.js
function testAPI() {
    fetch('/api/ping')
        .then(response => response.text())
        .then(data => alert('API says: ' + data));
}
```

## Integration with Routes

Static files work seamlessly with your API routes:

```lua
-- config.lua
ROUTES = {
    ['api/users'] = {
        GET = { SQL = "get_users.sql" }
    }
    -- No route for '/app.js' - served statically from public/app.js
}
```

With this setup:
- `GET /api/users` → runs your SQL function
- `GET /app.js` → serves `public/app.js`
- `GET /` → serves `public/index.html`

## Best Practices

1. **Organize with folders**: Use `public/css/`, `public/js/`, `public/images/` for better organization
2. **Avoid spaces**: While supported, filenames without spaces are easier to work with
3. **Keep it secure**: Never put sensitive files in `public/` - they're accessible to anyone
4. **Optimize assets**: Compress images and minify CSS/JS for better performance

## Common Patterns

### SPA with API Backend
```
public/
├── index.html          # Your single-page app
├── app.js             # Frontend JavaScript
└── styles.css         # Styles

# Routes handle API calls
ROUTES = {
    ['api/*'] = { ... }  # All API routes
}
```

### Mixed Static + Dynamic
```lua
ROUTES = {
    ['admin/dashboard'] = {
        GET = { 
            SQL = "admin_data.sql",
            VIEW = { { TYPE = "TEMPLATE" } }
        }
    }
}
# public/admin/login.html served statically
# /admin/dashboard route generates dynamic content
```

Static file serving in Pico just works - drop files in `public/` and they're instantly available on your web server.