--- @meta

--- @alias System "Black" | "Red" | "Green" | "Yellow" | "Blue" | "Cyan" | "Magenta" | "White"

--- @alias Color "System" | "Xterm" | "Rgb"

--- @class Example
--- @field color Color
local _CLASS_Example_ = {
	--- @param param1 string
	--- @param ... any
	logAny = function(param1, ...) end,
	--- @param param1 any
	printAll = function(param1) end,
	__metatable = {
		--- @param self Example
		--- @return string
		__tostring = function(self) end,
	}
}

--- @type Example
example = nil

--- Greet someone
--- @param name string Name of the person to greet
function greet(name) end

--- @param color Color Color to print to stdout
function printColor(color) end

