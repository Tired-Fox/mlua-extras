--- @meta

--- @alias System "Black" | "Red" | "Green" | "Yellow" | "Blue" | "Cyan" | "Magenta" | "White"

--- @alias Color "System" | "Xterm" | "Rgb"

--- This is a doc comment section for the overall type
--- @class Example
--- @field color Color
local _CLASS_Example_ = {
	--- Log a specific format with any lua types
	--- @param format string
	--- @param ... any
	logAny = function(format, ...) end,
	--- print all items
	--- @param ... any All of the args
	printAll = function(...) end,
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

