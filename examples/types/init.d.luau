export type System = "Black" | "Red" | "Green" | "Yellow" | "Blue" | "Cyan" | "Magenta" | "White"

export type Color = "System" | "Xterm" | "Rgb"

-- This is a doc comment section for the overall type
export type Example = {
	color: Color,
	-- Log a specific format with any lua types
	logAny: (format: string, ...: any) -> (),
	-- print all items
	printAll: (...: any) -> (),
	__tostring: (self: Example) -> string,
}

declare example: Example

-- Greet someone
-- @param name string -- Name of the person to greet
declare function greet(name: string): ()

-- @param color Color -- Color to print to stdout
declare function printColor(color: Color): ()
