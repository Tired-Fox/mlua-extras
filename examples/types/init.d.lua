--- @meta

--- This is a doc comment section for the overall type
--- @class Example
--- Example complex type
--- @field color Color
local _Class_Example = {
	--- print the Example userdata
	--- @param self Example
	print = function(self) end,
	__metatable = {
		--- @param self Example
		--- @return string
		__tostring = function(self) end,
	},
}

--- @alias Color "Black"
---  | "Red"
---  | "Green"
---  | "Yellow"
---  | "Blue"
---  | "Cyan"
---  | "Magenta"
---  | "White"
---  | integer
---  | { [1]: integer, [2]: integer, [3]: integer }

--- Example module
--- @type Example
example = nil

--- Options
--- @alias options "literal"

--- Say hello to someone
--- @param param0 string
function hello(param0) end
