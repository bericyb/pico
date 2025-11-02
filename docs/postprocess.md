# POSTPROCESS Handler

The POSTPROCESS handler is executed after the SQL handler and allows you to transform, format, and enhance the database response before it's sent to the client or passed to the next handler in the pipeline.

## What is POSTPROCESS for?

POSTPROCESS is your opportunity to shape the final response. Use it when you need to:
- Transform SQL results into client-friendly formats
- Add computed fields or metadata
- Filter or sanitize sensitive data
- Format data for specific use cases
- Combine multiple SQL results
- Handle error cases and provide meaningful messages

## Signature

```lua
POSTPROCESS = function(resp)
    -- Your logic here
    return modified_resp
end

-- Or with optional JWT parameter
POSTPROCESS = function(resp, jwt)
    -- Your logic here with access to current JWT
    return modified_resp
end
```

The function receives:
- `resp`: The SQL handler's response as its primary input
- `jwt` (optional): The current JWT claims if a user is authenticated

It must return the (potentially modified) response that will be passed to the next handler or sent as the final response.

## Examples

### Basic Data Transformation
```lua
POSTPROCESS = function(resp)
    -- Convert timestamps to human-readable format
    if resp.created_at then
        resp.created_at_formatted = os.date("%Y-%m-%d %H:%M:%S", resp.created_at)
    end
    
    return resp
end
```

### Adding Computed Fields
```lua
POSTPROCESS = function(resp)
    -- Add full name from first and last name
    if resp.first_name and resp.last_name then
        resp.full_name = resp.first_name .. " " .. resp.last_name
    end
    
    -- Calculate age from birth_date
    if resp.birth_date then
        local birth_year = tonumber(string.sub(resp.birth_date, 1, 4))
        resp.age = os.date("%Y") - birth_year
    end
    
    return resp
end
```

### Filtering Sensitive Data
```lua
POSTPROCESS = function(resp)
    -- Remove sensitive fields before sending to client
    resp.password_hash = nil
    resp.internal_notes = nil
    resp.ssn = nil
    
    return resp
end
```

### Error Handling and Messaging
```lua
POSTPROCESS = function(resp)
    -- Handle empty results
    if resp == nil or (type(resp) == "table" and #resp == 0) then
        return {
            success = false,
            message = "No data found",
            data = {}
        }
    end
    
    -- Wrap successful response
    return {
        success = true,
        message = "Data retrieved successfully",
        data = resp
    }
end
```

### Array Processing
```lua
POSTPROCESS = function(resp)
    -- Process array of users
    if type(resp) == "table" and #resp > 0 then
        for i, user in ipairs(resp) do
            -- Add display name
            user.display_name = user.first_name .. " " .. user.last_name
            
            -- Format email
            user.email = string.lower(user.email)
            
            -- Add profile URL
            user.profile_url = "/profile/" .. user.id
        end
    end
    
    return resp
end
```

### Complex Response Formatting
```lua
POSTPROCESS = function(resp)
    -- Transform flat SQL result into nested structure
    local formatted = {
        user = {
            id = resp.user_id,
            name = resp.user_name,
            email = resp.user_email
        },
        profile = {
            bio = resp.bio,
            avatar = resp.avatar_url,
            preferences = {
                theme = resp.theme,
                notifications = resp.notifications_enabled
            }
        },
        stats = {
            login_count = resp.login_count,
            last_login = resp.last_login
        }
    }
    
    return formatted
end
```

### Conditional Response Modification
```lua
POSTPROCESS = function(resp)
    -- Different handling based on response type
    if resp.user_type == "admin" then
        resp.permissions = {"read", "write", "delete", "admin"}
        resp.dashboard_url = "/admin"
    elseif resp.user_type == "moderator" then
        resp.permissions = {"read", "write", "moderate"}
        resp.dashboard_url = "/mod"
    else
        resp.permissions = {"read"}
        resp.dashboard_url = "/user"
    end
    
    return resp
end
```

### Response Aggregation
```lua
POSTPROCESS = function(resp)
    -- Calculate summary statistics
    if type(resp) == "table" and #resp > 0 then
        local total = 0
        local count = #resp
        
        for _, item in ipairs(resp) do
            total = total + (item.amount or 0)
        end
        
        return {
            items = resp,
            summary = {
                total_amount = total,
                average_amount = total / count,
                item_count = count
            }
        }
    end
    
    return resp
end
```

### JWT-Aware Response Filtering
```lua
POSTPROCESS = function(resp, jwt)
    -- Filter response based on user permissions
    if jwt and jwt.role == "admin" then
        -- Admins see everything
        return resp
    elseif jwt and jwt.role == "user" then
        -- Regular users don't see sensitive admin fields
        if type(resp) == "table" then
            resp.internal_notes = nil
            resp.admin_flags = nil
        end
    else
        -- Anonymous users get minimal data
        if type(resp) == "table" then
            resp = {
                id = resp.id,
                name = resp.name,
                public_info = resp.public_info
            }
        end
    end
    
    return resp
end
```

### User-Scoped Response Enhancement
```lua
POSTPROCESS = function(resp, jwt)
    if jwt then
        -- Add user-specific computed fields
        if type(resp) == "table" and resp.id then
            resp.is_owner = (resp.created_by == jwt.user_id)
            resp.can_edit = (resp.created_by == jwt.user_id) or (jwt.role == "admin")
            resp.user_context = {
                current_user_id = jwt.user_id,
                viewing_as = jwt.username
            }
        end
    end
    
    return resp
end
```

### Personalized Response Data
```lua
POSTPROCESS = function(resp, jwt)
    -- Customize response based on authenticated user
    if jwt and type(resp) == "table" then
        -- Add personalized URLs
        resp.profile_url = "/profile/" .. jwt.user_id
        resp.settings_url = "/settings"
        
        -- Add user preferences to response
        resp.user_preferences = {
            theme = jwt.theme or "light",
            timezone = jwt.timezone or "UTC"
        }
        
        -- Mark user's own content
        if resp.items then
            for _, item in ipairs(resp.items) do
                item.is_mine = (item.user_id == jwt.user_id)
            end
        end
    end
    
    return resp
end
```

## JWT Parameter Support

**New in Pico**: POSTPROCESS handlers now support an optional JWT parameter for accessing current user authentication state.

### Backward Compatibility
Both signature patterns are fully supported:

```lua
-- Legacy signature (still fully supported)
POSTPROCESS = function(resp)
    return resp
end

-- New signature with JWT access
POSTPROCESS = function(resp, jwt)
    -- Access current user authentication for response customization
    if jwt then
        resp.user_id = jwt.user_id
        resp.is_authenticated = true
    end
    return resp
end
```

The framework automatically detects whether your function expects 1 or 2 parameters and calls it appropriately. Existing handlers will continue to work unchanged.

## String Formatting and Templates
```lua
POSTPROCESS = function(resp)
    -- Create formatted messages
    if resp.id then
        resp.success_message = string.format(
            "User %s (ID: %d) was successfully created at %s",
            resp.username,
            resp.id,
            resp.created_at
        )
    end
    
    return resp
end
```

## Best Practices

1. **Always Return Something**: Even if you don't modify the response, return it
2. **Handle Nil Cases**: Check for nil or empty responses gracefully
3. **Preserve Data Types**: Be mindful of converting between types
4. **Security First**: Remove sensitive data before sending responses
5. **Keep Logic Simple**: Complex business logic might belong in SQL or PREPROCESS
6. **Document Transformations**: Comment complex data transformations

## Integration with Other Handlers

POSTPROCESS works with the complete handler pipeline:

```lua
{
    PREPROCESS = function(req)
        return { user_id = req.id }
    end,
    SQL = "get_user_profile.sql",
    POSTPROCESS = function(resp)
        -- Remove sensitive data
        resp.password_hash = nil
        
        -- Add computed fields
        resp.profile_complete = (resp.bio and resp.avatar) and true or false
        
        return resp
    end,
    SETJWT = function(resp, jwt)
        -- JWT handler receives the postprocessed response
        if resp.id then
            jwt.user_id = resp.id
            return jwt
        end
    end
}
```

## Common Patterns

### API Response Wrapper
```lua
POSTPROCESS = function(resp)
    return {
        status = "success",
        timestamp = os.time(),
        data = resp
    }
end
```

### Pagination Support
```lua
POSTPROCESS = function(resp)
    if type(resp) == "table" then
        return {
            items = resp,
            pagination = {
                total = #resp,
                page = 1,
                per_page = 20
            }
        }
    end
    return resp
end
```

### JWT-Aware Pagination
```lua
POSTPROCESS = function(resp, jwt)
    if type(resp) == "table" then
        local result = {
            items = resp,
            pagination = {
                total = #resp,
                page = 1,
                per_page = 20
            }
        }
        
        -- Add user-specific metadata
        if jwt then
            result.user_context = {
                user_id = jwt.user_id,
                can_create = jwt.permissions and 
                    (jwt.permissions.write or jwt.role == "admin")
            }
        end
        
        return result
    end
    return resp
end
```

POSTPROCESS is essential for creating clean, client-ready responses that hide implementation details and provide exactly the data your frontend needs.

