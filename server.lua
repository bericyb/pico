return {
  DB = 'postgresql://postgres:password@0.0.0.0:5432/pico',
  ROUTES = {
    [''] = {
      GET = {
        VIEW = {
          {
            TYPE = 'LINKS',
            LINKS = {
              { value = 'login', label = 'Login' },
              { value = 'register', label = 'Register' },
            },
          },
        },
      },
    },
    ['login'] = {
      GET = {
        VIEW = {
          {
            TYPE = 'POSTFORM',
            TITLE = 'Login',
            TARGET = '/login',
            FIELDS = {
              { id = 'username', type = 'text', label = 'Username' },
              { id = 'password', type = 'password', label = 'Password' },
              { id = 'button', type = 'submit', value = 'Login' },
            },
          },
        },
      },
      POST = {
        VIEW = {
          {
            TYPE = 'MARKDOWN',
          },
          {
            TYPE = 'LINKS',
            LINKS = {
              { value = '', label = 'Home' },
            },
          },
        },
        SQL = 'login.sql',
        PREPROCESS = function(params, jwt)
          print('Login PREPROCESS:', params, 'JWT:', jwt)
          if jwt and jwt.userId then
            print('User already authenticated as:', jwt.userId)
            -- Could redirect or modify params here
          end
          return params
        end,
        POSTPROCESS = function(obj, jwt)
          print('Login POSTPROCESS:', obj, 'JWT:', jwt)
          if obj and obj.id then
            if jwt then
              return 'Login successful (already authenticated as user ' .. (jwt.userId or 'unknown') .. ')'
            else
              return 'Login successful'
            end
          else
            return 'Invalid username or password'
          end
        end,
        SETJWT = function(obj, jwt)
          return {
            userId = obj.id,
          }
        end,
      },
    },
    ['test-jwt'] = {
      GET = {
        PREPROCESS = function(params, jwt)
          print('Test PREPROCESS - params:', params, 'JWT:', jwt)
          if jwt then
            print('JWT found - userId:', jwt.userId)
            params.authenticated_user = jwt.userId
          else
            print 'No JWT found'
            params.authenticated_user = nil
          end
          return params
        end,
        POSTPROCESS = function(obj, jwt)
          print('Test POSTPROCESS - obj:', obj, 'JWT:', jwt)
          if jwt then
            return {
              message = 'Hello authenticated user ' .. (jwt.userId or 'unknown'),
              jwt_present = true,
              user_id = jwt.userId,
            }
          else
            return {
              message = 'Hello anonymous user',
              jwt_present = false,
            }
          end
        end,
      },
    },
    ['ping'] = {
      POST = {
        SQL = 'pong.sql',
      },
      GET = {
        SQL = 'pong.sql',
      },
    },
    ['post'] = {
      POST = {
        SQL = 'createPost.sql',
      },
      GET = {
        VIEW = {
          {
            TYPE = 'MARKDOWN',
          },
          {
            TYPE = 'POSTFORM',
            TITLE = 'Create Post',
            TARGET = '/post',
            FIELDS = {
              { id = 'title', type = 'text', label = 'Title' },
              { id = 'content', type = 'textarea', label = 'Content' },
              { id = 'button', type = 'submit', value = 'Create Post', label = 'Create Post' },
            },
          },
        },
        SQL = 'getPosts.sql',
      },
    },
    ['logout'] = {
      POST = {
        SQL = 'logout.sql',
        SETJWT = function()
          return nil
        end,
      },
    },
  },
}
