--- @meta

--- @alias Color "Black" | "Red" | "Green" | "Yellow" | "Blue" | "White" | integer | { [1]: integer, [2]: integer, [3]: integer }

--- This is a doc comment section for the overall type
--- @class Example
--- Example complex type
--- @field color Color
--- print the Example userdata
--- @field print fun(self)

--- @param callback fun(example: Example) Do something with an example
function hello(callback) end

example = {
	nested = {
		--- @type Color
		color = nil,
	},
}
