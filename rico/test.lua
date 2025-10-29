return {
  DB = 'postgresql://postgres:password@0.0.0.0:5432/pico',
  ROUTES = {
    ['test'] = {
      GET = {
        PREPROCESS = function(params, jwt)
          print('PREPROCESS called with params:', params, 'JWT:', jwt)
          if jwt then
            print('JWT userId:', jwt.userId)
          else
            print('No JWT provided')
          end
          return params
        end,
        POSTPROCESS = function(obj, jwt)
          print('POSTPROCESS called with obj:', obj, 'JWT:', jwt)
          local result = {
            message = "Test successful",
            jwt_present = jwt ~= nil
          }
          if jwt and jwt.userId then
            result.user_id = jwt.userId
            result.message = "Test successful for user " .. jwt.userId
          end
          return result
        end,
      },
    },
  },
}