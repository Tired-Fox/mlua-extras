--- Test module documentation
test = {
	--- Some test data
	--- @type string
	data = nil,
	--- Nested module
	nested = {},
	--- Greetings
	--- @param name string Name of the person to greet
	greet = function(name) end,
	__metatable = {
		--- Meta field
		--- @type integer
		__count = nil,
	},
}

test.greet("Zachary")
