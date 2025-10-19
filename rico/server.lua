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
        POSTPROCESS = function(obj)
          print('Login POSTPROCESS:', obj)
          if obj and obj.id then
            return 'Login successful'
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
    ['workout'] = {
      POST = {
        SQL = 'createWorkout.sql',
      },
      GET = {
        SQL = 'getWorkouts.sql',
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
    ['user/:id/profile'] = {
      GET = {
        SQL = 'getUser.sql',
      },
      PUT = {
        SQL = 'updateUser.sql',
      },
      POST = {
        SQL = 'createUser.sql',
      },
      DELETE = {
        SQL = 'deleteUser.sql',
      },
    },
    ['team'] = {
      GET = {
        SQL = 'getTeam.sql',
      },
      PUT = {
        SQL = 'updateTeam.sql',
      },
      POST = {
        SQL = 'createTeam.sql',
      },
      DELETE = {
        SQL = 'deleteTeam.sql',
      },
    },
  },
}
