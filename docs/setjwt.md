# SETJWT Handler

The SETJWT handler manages JSON Web Tokens (JWTs) for authentication and session management. It runs after POSTPROCESS and allows you to create, update, or invalidate JWTs based on the response data and current authentication state.

## What is SETJWT for?

SETJWT handles all aspects of JWT-based authentication in your Pico application. Use it to:
- Log users in by creating new JWTs
- Update existing JWT claims with new information
- Log users out by invalidating tokens
- Add permissions or roles to user sessions
- Refresh token data based on database changes
- Implement session persistence and management

## Signature

```lua
SETJWT = function(resp, jwt)
    -- Your logic here
    return new_jwt_claims -- or nil to clear JWT
end
```

The function receives:
- `resp`: The response from POSTPROCESS (or SQL if no POSTPROCESS)
- `jwt`: Current JWT claims (nil if no token exists)

And returns:
- A table of new JWT claims to set
- `nil` to clear/invalidate the current JWT
- The existing `jwt` parameter to keep current token unchanged

## Examples

### User Login
```lua
SETJWT = function(resp, jwt)
    -- Create JWT after successful login
    if resp.id then
        return {
            user_id = resp.id,
            username = resp.username,
            user_type = resp.user_type,
            login_time = os.time()
        }
    end
    
    -- No user found, clear any existing JWT
    return nil
end
```

### User Logout
```lua
SETJWT = function(resp, jwt)
    -- Always clear JWT on logout
    return nil
end
```

### Updating User Permissions
```lua
SETJWT = function(resp, jwt)
    if jwt and resp.new_permissions then
        -- Update existing JWT with new permissions
        jwt.permissions = resp.new_permissions
        jwt.updated_at = os.time()
        return jwt
    end
    
    return jwt -- No changes
end
```

### Role-Based Access
```lua
SETJWT = function(resp, jwt)
    if resp.user_id then
        local claims = {
            user_id = resp.user_id,
            username = resp.username,
            role = resp.role
        }
        
        -- Add role-specific claims
        if resp.role == "admin" then
            claims.admin_level = resp.admin_level
            claims.permissions = {"read", "write", "delete", "admin"}
        elseif resp.role == "moderator" then
            claims.permissions = {"read", "write", "moderate"}
        else
            claims.permissions = {"read"}
        end
        
        return claims
    end
    
    return nil
end
```

### Session Refresh
```lua
SETJWT = function(resp, jwt)
    if jwt then
        -- Refresh session data from database
        jwt.last_activity = os.time()
        jwt.username = resp.current_username -- In case user changed name
        jwt.profile_complete = resp.profile_complete
        
        return jwt
    end
    
    return nil
end
```

### Conditional Authentication
```lua
SETJWT = function(resp, jwt)
    -- Only authenticate if account is active
    if resp.id and resp.status == "active" then
        return {
            user_id = resp.id,
            username = resp.username,
            account_status = resp.status,
            login_time = os.time()
        }
    elseif resp.id and resp.status == "suspended" then
        -- User exists but account is suspended
        resp.error = "Account is suspended"
        return nil
    end
    
    -- Login failed
    return nil
end
```

### JWT with Expiration Management
```lua
SETJWT = function(resp, jwt)
    if resp.user_id then
        local now = os.time()
        
        return {
            user_id = resp.user_id,
            username = resp.username,
            issued_at = now,
            expires_at = now + (24 * 60 * 60), -- 24 hours
            session_id = resp.session_id
        }
    end
    
    return nil
end
```

### Multi-Factor Authentication
```lua
SETJWT = function(resp, jwt)
    if resp.user_id and resp.password_valid then
        if resp.requires_2fa then
            -- Partial authentication - require 2FA
            return {
                user_id = resp.user_id,
                auth_state = "pending_2fa",
                temp_token = true,
                issued_at = os.time()
            }
        else
            -- Full authentication
            return {
                user_id = resp.user_id,
                username = resp.username,
                auth_state = "fully_authenticated",
                login_time = os.time()
            }
        end
    end
    
    return nil
end
```

### Preserving Existing Claims
```lua
SETJWT = function(resp, jwt)
    if jwt then
        -- Keep existing JWT but update last activity
        jwt.last_activity = os.time()
        
        -- Add new data from response if available
        if resp.new_notification_count then
            jwt.notification_count = resp.new_notification_count
        end
        
        return jwt
    end
    
    return nil
end
```

## JWT Claims Best Practices

### Standard Claims
```lua
{
    user_id = 123,           -- Primary identifier
    username = "john_doe",   -- Display name
    issued_at = os.time(),   -- When token was created
    expires_at = os.time() + 3600, -- When token expires
    session_id = "abc123"    -- Session identifier
}
```

### Role and Permission Claims
```lua
{
    user_id = 123,
    role = "admin",
    permissions = {"read", "write", "delete"},
    organization_id = 456,
    department = "engineering"
}
```

## Error Handling

Handle authentication failures gracefully:

```lua
SETJWT = function(resp, jwt)
    -- Check for authentication errors
    if resp.error then
        -- Log the error, clear JWT
        print("Authentication error: " .. resp.error)
        return nil
    end
    
    -- Validate required fields
    if not resp.user_id or not resp.username then
        print("Invalid authentication response")
        return nil
    end
    
    return {
        user_id = resp.user_id,
        username = resp.username
    }
end
```

## Security Considerations

1. **Minimal Claims**: Only include necessary data in JWT
2. **Sensitive Data**: Never put passwords or sensitive data in JWT
3. **Expiration**: Always consider token expiration
4. **Validation**: Validate all data before adding to JWT
5. **Logging**: Log authentication events for security auditing

## Integration with Complete Pipeline

```lua
{
    PREPROCESS = function(req)
        return { 
            username = string.lower(req.username),
            password = req.password 
        }
    end,
    SQL = "authenticate_user.sql",
    POSTPROCESS = function(resp)
        if resp.id then
            resp.login_successful = true
        else
            resp.login_successful = false
            resp.error = "Invalid credentials"
        end
        return resp
    end,
    SETJWT = function(resp, jwt)
        if resp.login_successful then
            return {
                user_id = resp.id,
                username = resp.username,
                role = resp.role,
                login_time = os.time()
            }
        end
        return nil -- Clear any existing JWT on failed login
    end,
    VIEW = {
        { TYPE = "MARKDOWN" }
    }
}
```

## Common Patterns

### Login Route
```lua
SETJWT = function(resp, jwt)
    if resp.user_id then
        return { user_id = resp.user_id, username = resp.username }
    end
    return nil
end
```

### Protected Route
```lua
SETJWT = function(resp, jwt)
    -- Keep existing authentication, just refresh activity
    if jwt then
        jwt.last_activity = os.time()
    end
    return jwt
end
```

### Logout Route
```lua
SETJWT = function(resp, jwt)
    return nil -- Always clear JWT
end
```

SETJWT is crucial for maintaining secure, stateful user sessions while keeping your authentication logic centralized and manageable.