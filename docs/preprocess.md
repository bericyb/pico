# PREPROCESS Handler

The PREPROCESS handler is the first handler in Pico's request processing pipeline. It allows you to transform, validate, and prepare incoming request data before it reaches your SQL handler.

## What is PREPROCESS for?

PREPROCESS is your gateway for handling incoming requests. Use it when you need to:
- Validate request data before hitting the database
- Transform request bodies into the format your SQL expects
- Add computed fields or default values
- Sanitize user input
- Extract data from complex request structures

## Signature

```lua
PREPROCESS = function(req)
    -- Your logic here
    return modified_req
end

-- Or with optional JWT parameter
PREPROCESS = function(req, jwt)
    -- Your logic here with access to current JWT
    return modified_req
end
```

The function receives:
- `req`: The request body as its primary input
- `jwt` (optional): The current JWT claims if a user is authenticated

It must return the (potentially modified) request body that will be passed to the SQL handler.

## Examples

### Basic Validation
```lua
PREPROCESS = function(req)
    if req.email == nil or req.email == "" then
        error("Email is required")
    end
    return req
end
```

### Data Transformation
```lua
PREPROCESS = function(req)
    -- Convert email to lowercase
    req.email = string.lower(req.email)
    
    -- Add timestamp
    req.created_at = os.time()
    
    -- Hash password (in real apps, use proper hashing)
    req.password_hash = hash(req.password)
    req.password = nil -- Remove plain password
    
    return req
end
```

### Complex Data Extraction
```lua
PREPROCESS = function(req)
    -- Extract nested data
    if req.user and req.user.profile then
        req.first_name = req.user.profile.first_name
        req.last_name = req.user.profile.last_name
    end
    
    -- Set defaults
    req.status = req.status or "active"
    
    return req
end
```

### JWT-Aware Validation
```lua
PREPROCESS = function(req, jwt)
    -- Check if user is authenticated
    if jwt == nil then
        error("Authentication required")
    end
    
    -- Add user context to request
    req.user_id = jwt.user_id
    req.requesting_user = jwt.username
    
    return req
end
```

### Role-Based Request Processing
```lua
PREPROCESS = function(req, jwt)
    -- Different validation based on user role
    if jwt and jwt.role == "admin" then
        -- Admins can process any request
        return req
    elseif jwt and jwt.role == "user" then
        -- Regular users can only modify their own data
        if req.target_user_id and req.target_user_id ~= jwt.user_id then
            error("You can only modify your own data")
        end
        req.user_id = jwt.user_id
    else
        error("Invalid user role")
    end
    
    return req
end
```

### Request Scoping with JWT
```lua
PREPROCESS = function(req, jwt)
    if jwt then
        -- Automatically scope requests to authenticated user
        req.created_by = jwt.user_id
        req.organization_id = jwt.organization_id
        
        -- Add audit fields
        req.modified_by = jwt.user_id
        req.modified_at = os.time()
    else
        -- Handle anonymous requests
        req.is_anonymous = true
    end
    
    return req
end
```

### JWT-Aware Validation
```lua
PREPROCESS = function(req, jwt)
    -- Check if user is authenticated
    if jwt == nil then
        error("Authentication required")
    end
    
    -- Add user context to request
    req.user_id = jwt.user_id
    req.requesting_user = jwt.username
    
    return req
end
```

### Role-Based Request Processing
```lua
PREPROCESS = function(req, jwt)
    -- Different validation based on user role
    if jwt and jwt.role == "admin" then
        -- Admins can process any request
        return req
    elseif jwt and jwt.role == "user" then
        -- Regular users can only modify their own data
        if req.target_user_id and req.target_user_id ~= jwt.user_id then
            error("You can only modify your own data")
        end
        req.user_id = jwt.user_id
    else
        error("Invalid user role")
    end
    
    return req
end
```

### Request Scoping with JWT
```lua
PREPROCESS = function(req, jwt)
    if jwt then
        -- Automatically scope requests to authenticated user
        req.created_by = jwt.user_id
        req.organization_id = jwt.organization_id
        
        -- Add audit fields
        req.modified_by = jwt.user_id
        req.modified_at = os.time()
    else
        -- Handle anonymous requests
        req.is_anonymous = true
    end
    
    return req
end
```

### Format Transformation
```lua
PREPROCESS = function(req)
    -- Transform frontend format to database format
    local db_req = {
        user_name = req.username,
        user_email = req.email,
        user_age = tonumber(req.age) or 0,
        preferences = {
            theme = req.theme or "light",
            notifications = req.notifications or true
        }
    }
    
    return db_req
end
```

## JWT Parameter Support

**New in Pico**: PREPROCESS handlers now support an optional JWT parameter for accessing current user authentication state.

### Backward Compatibility
Both signature patterns are fully supported:

```lua
-- Legacy signature (still fully supported)
PREPROCESS = function(req)
    return req
end

-- New signature with JWT access
PREPROCESS = function(req, jwt)
    -- Access current user authentication
    if jwt then
        req.user_id = jwt.user_id
    end
    return req
end
```

The framework automatically detects whether your function expects 1 or 2 parameters and calls it appropriately. Existing handlers will continue to work unchanged.

## Error Handling

If validation fails or an error occurs, you can use Lua's `error()` function to halt processing:

```lua
PREPROCESS = function(req)
    if not req.username or #req.username < 3 then
        error("Username must be at least 3 characters")
    end
    
    if not req.password or #req.password < 8 then
        error("Password must be at least 8 characters")
    end
    
    return req
end
```

## Best Practices

1. **Keep it Simple**: PREPROCESS should focus on data preparation, not business logic
2. **Validate Early**: Catch invalid data before it reaches your database
3. **Return Consistently**: Always return a request body, even if unchanged
4. **Document Transformations**: Complex data transformations should be well-commented
5. **Fail Fast**: Use `error()` for validation failures to provide clear feedback

## Integration with Other Handlers

PREPROCESS works seamlessly with other handlers in the pipeline:

```lua
{
    PREPROCESS = function(req)
        req.email = string.lower(req.email)
        return req
    end,
    SQL = "create_user.sql", -- Receives the processed request
    POSTPROCESS = function(resp)
        return "User created with ID: " .. resp.id
    end
}
```

The processed request from PREPROCESS becomes the input to your SQL function, making data flow predictable and clean.

