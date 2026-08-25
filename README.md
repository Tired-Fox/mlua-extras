# MLua Extras

> [!NOTE]
> Feel free to use this crate and start working with ideas and features that could be useful.
>
> Pull requests and contribution are encouraged
>
> If you want to discuss this project, you can do that [here](https://github.com/Tired-Fox/mlua-extras/discussions/1)

___

The goal of this project is to add a light convenience layer wrapping the [`mlua`](https://docs.rs/mlua/latest/mlua/) crate. The goal isn't to change the way that `mlua` is used, but instead to make `lua` embedded development in `Rust` more enjoyable.

## Similar Projects

- `Tealr`: A project to enhance and extend `mlua` with a focus in type information and documentation along with a type syntax in the lua code itself with the `tealr` syntax.
    - This crate is a great choice if you need: type syntax, type information, documentation generation

## Features

- Helper Traits
    - `LuaExtras`
        - Manipulate the lua [`path`](https://www.lua.org/manual/5.1/manual.html#pdf-package.path) and [`cpath`](https://www.lua.org/manual/5.1/manual.html#pdf-package.cpath) variables with `append`, `prepend`, and `set` methods for each variant. It also includes the ability to add multiple paths with each variant.
        - Set global variables and functions with `set_global("value", "value")` and `set_global_function("func", |lua, ()| Ok(()))` which wold replace `lua.globals().set("value", "value)` and `lua.globals().set("func", lua.create_function(|lua, ()| Ok(()))?)` respectively

- Typed Lua Traits
    - `Typed`
        - Generate a `Type` and `Param` for a rust type so it can be used both as a type and as a parameter for a function
    - `TypedUserData`
        - Typed variant of [`mlua::UserData`](https://docs.rs/mlua/latest/mlua/trait.UserData.html) with an additional `add_documentation` method to add doc comments to the [`UserData`](https://docs.rs/mlua/latest/mlua/trait.UserData.html) type
        - An extra `document` method is added to the `TypedDataFields` and `TypedDataMethods` for [`add_fields`](https://docs.rs/mlua/latest/mlua/trait.UserData.html#method.add_fields) and [`add_methods`](https://docs.rs/mlua/latest/mlua/trait.UserData.html#method.add_methods). This will queue doc comments to be added to the next field or method that is added.
        - All types from function parameters and and return types are stored for fields, functions, and methods.
        - This trait is mainly used when generating type definitions. If it is called through the [`UserData`](https://docs.rs/mlua/latest/mlua/trait.UserData.html) derive macro it will ignore all types and documentation
    - `TypedDataFields`: Implemented on a generator for `TypedUserData` ([`add_fields`](https://docs.rs/mlua/latest/mlua/trait.UserData.html#method.add_fields))
    - `TypedDataMethods`: Implemented on a generator for `TypedUserData` ([`add_methods`](https://docs.rs/mlua/latest/mlua/trait.UserData.html#method.add_methods))
    - `TypedDataDocumentation`: Implemented on a generator for `TypedUserData` (`add_documentation`)

- Derive Macros
    - `Typed`: Auto implement the `Typed` trait to get type information for both `struct` and `enum`
    - `UserData`: Auto implement the [`mlua::UserData`](https://docs.rs/mlua/latest/mlua/trait.UserData.html) trait for rust types that also implement `TypedUserData`. This will pass through the [`UserData`](https://docs.rs/mlua/latest/mlua/trait.UserData.html) [`add_methods`](https://docs.rs/mlua/latest/mlua/trait.UserData.html#method.add_methods) and [`add_fields`](https://docs.rs/mlua/latest/mlua/trait.UserData.html#method.add_fields) to the `TypedUserData`'s version. This will ignore all documentation and types.

## Ideas and Planned Features

- Fully featured definition file generation
- Fully featured documentation generation
- Fully featured addon generator when creating a lua modules with `mlua`'s `module` feature
- Better and more informative type errors associated with lua type definitions and output generation
- More expressive way of defining exposed lua api types
    - Generic types
    - Doc comments for params and return types

## References

- [`lua`](https://www.lua.org/)
- [`mlua`](https://github.com/mlua-rs/mlua)
- [`Tealr`](https://github.com/lenscas/tealr)
- [`Luau`](https://luau.org/)
- [`Lua Language Server`](https://github.com/LuaLS/lua-language-server)

## Example Syntax

**Helpers**

```rust
use mlua::{Lua, Table, Function, Variadic, Value};

fn main() -> mlua::Result<()> {
    let lua = Lua::new();

    // Prepend path to the lua `path`
    let path = lua.globals().get::<Table>("package")?.get::<String>("path");
    lua.globals().get::<Table>("package")?.set("path", format!("?.lua;{path}"))?;

    let temp = lua.create_table()?;
    temp.set("getName", lua.create_function(|lua, ()| Ok("name"))?;

    // Get a nested function: `table.unpack`
    let unpack = lua.globals().get::<Table>("table")?.get::<_, Function>("unpack")?;
    // Call the `table.unpack` function
    let _ = unpack.call::<Variadic<Value>>(temp)?;
    Ok(())
}
```

```rust
use mlua_extras::{
    mlua::{self, Lua, Table, Variadic, Value}
    extras::{Require, LuaExtras},
    typed::TypedFunction,
    function,
};

fn main() -> mlua::Result<()> {
    let lua = Lua::new();

    // Prepend path to the lua `path`
    lua.prepend_path("?.lua")?;

    let temp = lua.create_table()?;
    temp.set("name", "MluaExtras")?;

    // Get a nested function: `table.unpack`
    let unpack = lua.require::<TypedFunction<Table, Variadic<Value>>>("table.unpack")?;
    // Call the `table.unpack` function
    let _ = unpack.call(temp)?;
    Ok(())
}
```

**Types**

```rust
use serde::Deserialize;
use mlua_extras::{
    mlua::{self, Lua, Table, Variadic, Value},
    extras::{ Require, LuaExtras },
    typed::{
        generator::{Definition, Definitions, DefinitionFileGenerator},
        TypedFunction, TypedUserData
    },
    Typed, UserData, function,
};

#[derive(Default, Debug, Clone, Copy, Typed, Deserialize)]
enum SystemColor {
    #[default]
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Cyan,
    Magenta,
    White,
}

#[derive(Debug, Clone, Copy, Typed, Deserialize)]
#[serde(untagged)]
enum Color {
    System(SystemColor),
    Xterm(u8),
    Rgb(u8, u8, u8),
}
impl Default for Color {
    fn default() -> Self {
        Color::System(SystemColor::default())
    }
}
impl<'lua> FromLua<'lua> for Color {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> mlua::prelude::LuaResult<Self> {
        match value {
            Value::UserData(data) => data.borrow::<Self>().map(|v| *v),
            // Use serde deserialize if not userdata
            other => lua.from_value(other),
        }
    }
}

#[derive(Debug, Clone, Copy, Typed, UserData, Deserialize)]
struct Example {
    color: Color
}
impl TypedUserData for Example {
    fn add_documentation<F: mlua_extras::typed::TypedDataDocumentation<Self>>(docs: &mut F) {
        docs.add("This is a doc comment section for the overall type");
    }

    fn add_fields<'lua, F: TypedDataFields<'lua, Self>>(fields: &mut F) {
        fields
            .document("Example complex type")
            .add_field_method_get_set(
                "color",
                |_lua, this| Ok(this.color),
                |_lua, this, clr: Color| {
                    this.color = clr;
                    Ok(())
                },
            );
    }
}


fn main() -> mlua::Result<()> {
    let definitions = Definitions::generate()
        .define("init", Definition::generate()
            .register::<SystemColor>("System")?
            .register::<Color>("Color")?
            .register::<Example>("Example")
            .document("Example module")
            .value::<Example>("example")
            .function::<Color, ()>("printColor", ())
            .document("Greet the name that was passed in")
            .param("name", "Name of the person to greet")
            .function::<String, ()>("greet", ())
        )
        .finish();

    let gen = DefinitionFileGenerator::new(definitions);
    for (name, writer) in gen.iter() {
        // Writes to a new file `init.d.lua`
        writer.write_file(name).unwrap();
    }
    println!();
    Ok(())
}
```

Produces the following definition file

```lua
--- init.d.lua
--- @meta

--- @alias System SystemBlack
--- | SystemRed
--- | SystemGreen
--- | SystemYellow
--- | SystemBlue
--- | SystemCyan
--- | SystemMagenta
--- | SystemWhite

--- @class _System

--- @class SystemBlack: _System
--- @class SystemRed: _System
--- @class SystemGreen: _System
--- @class SystemYellow: _System
--- @class SystemBlue: _System
--- @class SystemCyan: _System
--- @class SystemMagenta: _System
--- @class SystemWhite: _System

    System(SystemColor),
    Xterm(u8),
    Rgb(u8, u8, u8),
--- @alias Color ColorSystem | ColorXterm | ColorRgb

--- @class _Color

--- @class ColorSystem: _Color
--- @field [1] SystemColor

--- @class ColorXterm: _Color
--- @field [1] integer

--- @class ColorRgb: _Color
--- @field [1] integer
--- @field [2] integer
--- @field [3] integer

--- This is a doc comment section for the overall type
--- @class Example
--- Example complex type
--- @field color Color

--- Example module
--- @type Example
example = nil

--- Greet the name that was passed in
--- @param name string Name of the person to greet
function greet(name) end

--- @param param0 Color
function printColor(param0) end
```

## Macros

There are helper macros that make writing lua integrations simplier and less manual. There
are variants that support recording type information, and variants that just focus on making
the creation of custom userdata types simple.

```rust
use std::path::PathBuf;
use mlua_extras::{
    mlua::Value,
    TypedUserData,
    typed::generator::{
        Definition, DefinitionFileGenerator, Definitions, LuauDefinitionFileGenerator,
    },
    typeduserdata_impl,
};

/// Simple Counter
#[derive(Clone, TypedUserData)]
struct Counter { value: i64 }

#[typeduserdata_impl]
impl Counter {
    /// The default count
    const COUNT: usize = 10;

    /// Max count value
    #[lua(field, infallible)]
    fn max() -> i64 {
        i64::MAX
    }

    /// Min count value
    #[lua(field, name = "MIN", infallible)]
    fn min() -> i64 {
        0
    }

    /// Transform the value to `up`
    #[lua(infallible)]
    fn transform(
        &self,
        /// Macro defined param doc comment
        value: Value
    ) -> String {
        "up".into()
    }

    /// Direction of the counter
    #[lua(getter, name = "direction", infallible)]
    fn get_direction(&self) -> String {
        "up".into()
    }

    #[lua(setter, name = "direction", infallible)]
    fn set_direction(&mut self, dir: String) {
        println!("Direction: {dir}");
    }

    /// Get the current counter value
    #[lua(infallible)]
    fn get(&self) -> i64 { self.value }

    /// Increment the counter
    #[lua(infallible)]
    fn increment(&mut self) { self.value += 1 }

    /// Create a new table
    fn create_table(&self, lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
        lua.create_table()
    }

    /// String representation of the counter
    #[lua(meta, infallible)]
    fn __tostring(&self) -> String { format!("Counter({})", self.value) }

    // Requires the `async` feature
    // Must be accessed from lua code with an entry of `mlua::Chunk::eval_async` or `mlua::Chunk::exec_async`

    /// Fetch the global counter online
    async fn fetch(&self, lua: mlua::Lua, url: String) -> mlua::Result<String> {
        _ = lua;
        Ok(format!("fetched: {url}"))
    }
}

fn main() -> mlua::Result<()> {
    let definitions: Definitions = Definitions::start()
        .define("macros", Definition::start().register::<Counter>("Counter"))
        .finish();

    let types_path = PathBuf::from("examples/types");
    if !types_path.exists() {
        std::fs::create_dir_all(&types_path).unwrap();
    }

    let dfg = DefinitionFileGenerator::new(definitions.clone());
    for (name, writer) in dfg.iter() {
        println!("==== Generated \x1b[1;33mexample/types/{name}\x1b[0m ====");
        writer.write_file(types_path.join(name)).unwrap();
    }

    Ok(())
}
```

Results in the lua type definition

```lua
--- @meta

--- Simple Counter
--- @class Counter
--- Direction of the counter
--- @field direction string
--- @field value integer
local _CLASS_Counter_ = {
	--- The default count
	COUNT = 10,
	--- Min count value
	MIN = 0,
	--- Max count value
	max = 9223372036854775807,
	--- Create a new table
	--- @param self Counter
	--- @return table
	create_table = function(self) end,
	--- Fetch the global counter online
	--- @param self Counter
	--- @param url string
	--- @return string
	fetch = function(self, url) end,
	--- Get the current counter value
	--- @param self Counter
	--- @return integer
	get = function(self) end,
	--- Increment the counter
	--- @param self Counter
	increment = function(self) end,
    --- 
    --- @param value any Macro defined param doc comment
    transform = function(self, value) end,
	__metatable = {
		--- String representation of the counter
		--- @param self Counter
		--- @return string
		__tostring = function(self) end,
  }
}
```

## Testing

To run all the tests in one shot, use `cargo test --features luau,vendored,send,async,userdata-wrappers,macros`

Some features of this crate generate luau compatible definition files, or use
luau specific features.  To add an additional layer of validation to the tests
you can install [luau-lsp](https://github.com/JohnnyMorganz/luau-lsp) and
the tests will run the type checker, and fail if the results are not as
expected.

See [our luau docs](LUAU.md#validating-generated-definitions) for more
information on installing the lsp; there are pre-built binaries available
which makes it quick and painless.

The `TEST_LUAU` environment variable controls luau-lsp validation:

| Value | Behavior |
|---|---|
| *(unset)* | Auto-detect `luau-lsp` on `PATH`. If found, run validation; otherwise skip. |
| `0` | Skip validation entirely, even if `luau-lsp` is on `PATH`. |
| *path* | Use the given value as the `luau-lsp` binary path (e.g. `TEST_LUAU=/tmp/luau-lsp`). |

Some tests validate the LuaLS-format (`.d.lua`) definition files generated by
`DefinitionFileGenerator`. These tests use
[lua-language-server](https://github.com/LuaLS/lua-language-server), which
ships as a self-contained binary with no runtime dependencies. Pre-built
archives for Linux, macOS, and Windows are available on the
[releases page](https://github.com/LuaLS/lua-language-server/releases/latest).

To install on Linux x64:

```sh
VERSION=$(curl -sI https://github.com/LuaLS/lua-language-server/releases/latest \
  | grep -i location | grep -oE '[0-9]+\.[0-9]+\.[0-9]+')
curl -fsSL "https://github.com/LuaLS/lua-language-server/releases/download/${VERSION}/lua-language-server-${VERSION}-linux-x64.tar.gz" \
  | tar -xz -C ~/.local
export PATH="$HOME/.local/bin:$PATH"
```

The `TEST_LUALS` environment variable controls validation:

| Value | Behavior |
|---|---|
| *(unset)* | Auto-detect `lua-language-server` on `PATH`. If found, run validation; otherwise skip. |
| `0` | Skip validation entirely, even if `lua-language-server` is on `PATH`. |
| *path* | Use the given value as the binary path (e.g. `TEST_LUALS=/opt/lua-ls/bin/lua-language-server`). |

