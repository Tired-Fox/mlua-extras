local uv = require('luv')

-- Create a handle to a uv_timer_t
local timer = uv.new_timer()

-- This will wait 1000ms and then continue inside the callback
timer:start(1000, 0, function ()
  -- timer here is the value we passed in before from new_timer.

  print("Awake!")

  -- You must always close your uv handles or you'll leak memory
  -- We can't depend on the GC since it doesn't know enough about libuv.
  timer:close()
end)

print("Sleeping")

-- uv.run will block and wait for all events to run.
-- When there are no longer any active handles, it will return
uv.run()
