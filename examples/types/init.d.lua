--- @meta

--- @alias SystemColor "Black"
---  | "Red"
---  | "Green"
---  | "Yellow"
---  | "Blue"
---  | "Cyan"
---  | "Magenta"
---  | "White"

--- @alias Color SystemColor
---  | integer
---  | { [1]: integer, [2]: integer, [3]: integer }

--- This is a doc comment section for the overall type
--- @class Example
--- Example complex type
--- @field color Color
local _Class_Example = {
  --- print all items
  --- @param ... string
  printAll = function(...) end,
  __metatable = {
    --- @param self Example
    --- @return string
    __tostring = function(self) end,
  }
}

--- Example module
--- @type Example
example = nil

--- Greet the name that was passed in
--- @param param0 string
function greet(param0) end

--- Print a color and it's value
--- @param param0 Color
function printColor(param0) end

