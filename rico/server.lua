return {
  DB = 'postgresql://postgres:password@0.0.0.0:5432/pico',
  ROUTES = {
    [''] = {
      GET = {
        VIEW = {
          {
            TYPE = 'LINKS',
            FIELDS = {
              'login',
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
              { id = 'button', type = 'submit', value = 'Login', label = 'Login' },
            },
          },
        },
      },
      POST = {
        SQL = 'login.sql',
        SETJWT = function(obj, jwt)
          return {
            userId = obj.id,
            teamId = obj.teamId,
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
        POLICY = function(obj, jwt)
          if obj.id == jwt.userId then
            return true
          else
            return false
          end
        end,
      },
      PUT = {
        SQL = 'updateUser.sql',
        POLICY = function(obj, jwt)
          if obj.id == jwt.userId then
            return true
          else
            return false
          end
        end,
      },
      POST = {
        SQL = 'createUser.sql',
      },
      DELETE = {
        SQL = 'deleteUser.sql',
        POLICY = function(obj, jwt)
          if obj.id == jwt.userId then
            return true
          else
            return false
          end
        end,
      },
    },
    ['team'] = {
      GET = {
        SQL = 'getTeam.sql',
        POLICY = function(obj, jwt)
          if obj.id == jwt.teamId then
            return true
          else
            return false
          end
        end,
      },
      PUT = {
        SQL = 'updateTeam.sql',
        POLICY = function(obj, jwt)
          if obj.id == jwt.teamId then
            return true
          else
            return false
          end
        end,
      },
      POST = {
        SQL = 'createTeam.sql',
        POLICY = function()
          return true
        end,
      },
      DELETE = {
        SQL = 'deleteTeam.sql',
        POLICY = function(obj, jwt)
          if obj.id == jwt.teamId then
            return true
          else
            return false
          end
        end,
      },
    },
  },
}
