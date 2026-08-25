export type System = "Black" | "Red" | "Green" | "Yellow" | "Blue" | "Cyan" | "Magenta" | "White"

export type Color = "System" | "Xterm" | "Rgb"

export type Example = {
	color: Color,
	logAny: (param1: string, ...: any) -> (),
	printAll: (param1: any) -> (),
	__tostring: (self: Example) -> string,
}

declare example: Example

-- Greet someone
-- @param name string -- Name of the person to greet
declare function greet(name: string): ()

-- @param color Color -- Color to print to stdout
declare function printColor(color: Color): ()
