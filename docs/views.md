Views in Pico are a powerful way to define frontend UI components declaratively using Lua tables. They provide a simple but flexible system for building web interfaces without writing HTML directly.

## What are Views?

Views are UI component definitions that describe how to render frontend elements. They are defined as Lua tables within your route handlers and are automatically converted to HTML by the Pico framework. Views support various entity types including forms, links, markdown content, and data objects.

## View Structure

Views are defined as an array of entities, where each entity has a `TYPE` field that determines how it will be rendered:

```lua
VIEW = {
    {
        TYPE = "LINKS",
        LINKS = {
            { value = "login", label = "Login" },
            { value = "register", label = "Register" }
        }
    },
    {
        TYPE = "MARKDOWN"
    }
}
```

## Supported Entity Types

### 1. LINKS

Creates a collection of hyperlinks for navigation.

```lua
{
    TYPE = "LINKS",
    LINKS = {
        { value = "home", label = "Home" },
        { value = "profile", label = "My Profile" },
        { value = "settings", label = "Settings" }
    }
}
```

**Properties:**
- `value`: The URL path (required)
- `label`: Display text for the link (optional)

### 2. POSTFORM, PUTFORM, DELETEFORM

Creates forms with different HTTP methods for user input.

```lua
{
    TYPE = "POSTFORM",
    TITLE = "Create Account",
    TARGET = "/register",
    FIELDS = {
        { id = "username", type = "text", label = "Username" },
        { id = "email", type = "email", label = "Email Address" },
        { id = "password", type = "password", label = "Password" },
        { id = "submit", type = "submit", value = "Register" }
    }
}
```

**Properties:**
- `TYPE`: One of "POSTFORM", "PUTFORM", or "DELETEFORM"
- `TITLE`: Form title (optional)
- `TARGET`: URL where form data will be submitted (required)
- `FIELDS`: Array of form field definitions (required)

**Field Properties:**
- `id`: Unique identifier for the field (required)
- `type`: HTML input type (text, password, email, submit, etc.) (required)
- `label`: Display label for the field (optional)
- `value`: Default value for the field (optional)

### 3. MARKDOWN

Renders content as markdown. The data from your SQL response or POSTPROCESS function will be displayed as markdown content.

```lua
{
    TYPE = "MARKDOWN"
}
```

When using MARKDOWN entities, the response data will be rendered as markdown content. If the data is a string, it's rendered directly. If it's an object, it's displayed in a structured format.

### 4. OBJECT

Renders JSON objects in a structured display format.

```lua
{
    TYPE = "OBJECT"
}
```

### 5. TABLE

Renders data in tabular format. The table view automatically detects columns from your data structure, making it extremely flexible for displaying any array of objects.

```lua
{
    TYPE = "TABLE"
}
```

**How it works:**
- **For arrays of objects**: Automatically creates columns based on the keys of the first object
- **For single objects**: Displays as a single-row table
- **For primitive values**: Creates a simple "value" column

**Example data structures that work:**

```lua
-- Array of user objects
[
    { name = "John", email = "john@example.com", age = 30 },
    { name = "Jane", email = "jane@example.com", age = 25 }
]
-- Creates columns: name, email, age

-- Single object
{ name = "John", email = "john@example.com", age = 30 }
-- Creates a single-row table with columns: name, email, age

-- Array of simple values
["apple", "banana", "cherry"]
-- Creates a single "value" column
```

The TABLE view requires no configuration - simply set `TYPE = "TABLE"` and it will automatically adapt to your data structure.

## Complete Route Examples

Here are comprehensive examples showing how Views integrate with other route handlers:

### Login Form Example

```lua
['login'] = {
    GET = {
        -- Display login form
        VIEW = {
            {
                TYPE = 'POSTFORM',
                TITLE = 'Login',
                TARGET = '/login',
                FIELDS = {
                    { id = 'username', type = 'text', label = 'Username' },
                    { id = 'password', type = 'password', label = 'Password' },
                    { id = 'button', type = 'submit', value = 'Login' }
                }
            }
        }
    },
    POST = {
        SQL = 'login.sql',
        POSTPROCESS = function(obj)
            if obj and obj.id then
                return 'Login successful'
            else
                return 'Invalid username or password'
            end
        end,
        SETJWT = function(obj, jwt)
            return { userId = obj.id }
        end,
        -- Display result and navigation
        VIEW = {
            {
                TYPE = 'MARKDOWN'  -- Shows POSTPROCESS result
            },
            {
                TYPE = 'LINKS',
                LINKS = {
                    { value = '', label = 'Home' }
                }
            }
        }
    }
}
```

### Table View Example

```lua
['users'] = {
    GET = {
        SQL = 'get_users.sql',  -- Returns array of user objects
        VIEW = {
            {
                TYPE = 'TABLE'  -- Automatically creates table from user data
            },
            {
                TYPE = 'LINKS',
                LINKS = {
                    { value = 'user/new', label = 'Add New User' }
                }
            }
        }
    }
}
```

If your `get_users.sql` returns:
```json
[
    { "id": 1, "username": "john_doe", "email": "john@example.com", "created_at": "2023-01-15" },
    { "id": 2, "username": "jane_smith", "email": "jane@example.com", "created_at": "2023-01-20" }
]
```

The table view will automatically create a table with columns: id, username, email, created_at.

## View Rendering Process

1. **Data Flow**: Views receive data from SQL responses or POSTPROCESS functions
2. **Entity Processing**: Each entity in the VIEW array is processed according to its TYPE
3. **HTML Generation**: The framework converts entities to HTML
4. **Client Rendering**: The generated HTML is sent to the browser with styling and interactivity

## Best Practices

1. **Keep Views Simple**: Views are meant for rapid prototyping and simple UIs
2. **Combine Entity Types**: Use multiple entities in a single view for rich interfaces
3. **Leverage Data Flow**: Use POSTPROCESS functions to format data for view consumption
4. **Form Validation**: Handle validation in PREPROCESS functions before SQL execution
5. **Navigation Patterns**: Use LINKS entities to provide clear navigation paths

## Migration and Extensibility

Views in Pico are designed to be a starting point for frontend development. As your application grows, you can:

- Integrate with frontend frameworks by treating Pico as an API backend
- Use the underlying Postgres database with any other web framework

Views provide a quick way to get a functional web interface without frontend complexity, while maintaining the flexibility to evolve your application architecture as needed.

