--- @type thread
print("Hello world!")

--- @alias System "Black" | "Red" | "Green" | "Yellow" | "Blue" | "White"
--- @alias Rgb { r: integer, g: integer, b: integer } | { [1]: integer, [2]: integer, [3]: integer }
--- @alias Color System | integer | Rgb

--- @param color Color
function colorPrint(color)
	print(color)
end

colorPrint({ r = 255, g = 16, b = 8 })
