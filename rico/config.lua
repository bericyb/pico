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
              { id = 'email', type = 'email', label = 'Email' },
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
        SQL = 'authenticate_user.sql',
        PREPROCESS = function(params, jwt)
          print('Login PREPROCESS:', params, 'JWT:', jwt)
          if jwt and jwt.userId then
            print('User already authenticated as:', jwt.userId)
          end
          return params
        end,
        POSTPROCESS = function(obj, jwt)
          print('Login POSTPROCESS:', obj, 'JWT:', jwt)
          if obj and obj.id then
            return 'Login successful! Welcome back.'
          else
            return 'Invalid email or password. Please try again.'
          end
        end,
        SETJWT = function(obj, jwt)
          if obj and obj.id then
            return {
              userId = obj.id,
              email = obj.email,
            }
          end
          return nil
        end,
      },
    },
    ['register'] = {
      GET = {
        VIEW = {
          {
            TYPE = 'POSTFORM',
            TITLE = 'Register',
            TARGET = '/register',
            FIELDS = {
              { id = 'email', type = 'email', label = 'Email' },
              { id = 'password', type = 'password', label = 'Password' },
              { id = 'button', type = 'submit', value = 'Register' },
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
              { value = 'login', label = 'Login' },
              { value = '', label = 'Home' },
            },
          },
        },
        SQL = 'register_user.sql',
        POSTPROCESS = function(obj, jwt)
          print('Registration POSTPROCESS:', obj, 'JWT:', jwt)
          if obj and obj.id then
            return 'Registration successful! Please login with your new account.'
          else
            return 'Registration failed. Email may already be in use.'
          end
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
      GET = {
        SQL = 'pong.sql',
      },
    },
    ['logout'] = {
      POST = {
        SETJWT = function()
          return nil
        end,
      },
    },
  },
}