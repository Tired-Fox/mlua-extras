# MLua Extras

> [!WARNING]
> This crate is under heavy active development. All features are currently experimental and are subject to change partially or fully at any time without notice.

> [!NOTE]
> Feel free to use this crate and start workshopping ideas and features that could be useful.
>
> PR's and Changes are welcome; Some topics may inclue API reworks/restruture, additional type information, more type generators, etc...
>
> If you want to discuss this project, you can do that [here](https://github.com/Tired-Fox/mlua-extras/discussions/1)

Add helpers for common coding styles with [`mlua`](https://docs.rs/mlua/latest/mlua/).

The biggest part of this library is adding lua type information and doc comments. The typing is a light wrapper around [`UserData`](https://docs.rs/mlua/latest/mlua/trait.UserData.html) and its related traits [`UserDataFields`](https://docs.rs/mlua/latest/mlua/trait.UserDataFields.html) and [`UserDataMethods`](https://docs.rs/mlua/latest/mlua/trait.UserDataMethods.html). There is a Definition generator that allows you to group the type definitions (entries), commonly thought of as definition files. From this API a generator for definition files, documentation, or anything else could be built on top. This library will also provide a basic definition file generator.

## Inspiration

I really enjoy having types information when writing my code. However, it can be a pain to write and maintain the types for lua while also writing the rust code and rust types. This is where this libarary comes in. Using ideas from [`Tealr`](https://github.com/lenscas/tealr), this library adds a wrapper around [`mlua`]'s traits and types to automatically get type information and doc comments for rust types.

Not all the pain of maintenance is gone as you have to explicitly say which types are going to be used and where. However, it greatly reduces the pain point and keeps the types and documentation close to where the code is written.

The hope is that this library will provide an API to generate definition files, lsp addons, and lua api documentation.

While we are already creating an API this library also adds some usefull helper traits and macros to make writing rust code for the lua engine much easier.

## Pain Point

There is still one huge pain point with this library. Since it implements traits for the `mlua` crate, it requires that it is a dependency. The `mlua` crate will not compile unless a version of lua is provided. This is bit of problem since it will fail to compile if your project depends on it with one lua version and this crate depends on it with a different version. With this in mind, this crate re-exposes `mlua` and exposes most of it's features. With this the crate can model it's traits and implementations based on how you want to use lua.

Just make sure you use the exposed `mlua` crate through this crates API.

## Features

- Helper Traits
    - `Require`
        - call `require` which allows for a lua style call to get data from the lua engine.
        - ex: `table.require::<String>("nested.tables.name")?` == `local name = require('nested.tables').name` 
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

- Macros
    - `function`: Write lua functions more like Rust's syntax

    Instead of this:

    ```rust
    lua.create_function(|lua, ()| Ok(())) 
    ```

    You can now write this:

    ```rust
    function!{
        lua fn name(lua) {
            Ok(())
        }
    }
    ```

    The difference isn't huge, but it could make the syntax more readable.

    > [!NOTE]
    > This also helps with assigning functions to nested tables.

    > [!WARNING]
    > This requires the `LuaExtras` trait when adding functions to nested tables with `lua` as the starting point. This requires the `Require` trait when starting from any other table.

    ```rust
    lua.require::<Table>("nested.fn").set("name", lua.create_function(|lua, ()| Ok(())));

    // vs

    function! {
        lua fn lua::nested::fn.name(lua) {
            Ok(())
        }
    }
    ```

## Example

**Helpers**

```rust
use mlua::{Lua, Table, Function, Variadic, Value};

fn main() -> mlua::Result<()> {
    let lua = Lua::new();

    // Prepend path to the lua `path`
    let path = lua.globals().get::<_, Table>("package")?.get::<_, String>("path");
    lua.globals().get::<_, Table>("package")?.set("path", format!("?.lua;{path}"))?;

    let temp = lua.create_table()?;
    temp.set("getName", lua.create_function(|lua, ()| Ok("name"))?;

    // Get a nested function: `table.unpack`
    let unpack = lua.globals().get::<_, Table>("table")?.get::<_, Function>("unpack")?;
    // Call the `table.unpack` function
    let _ = unpack.call::<_, Variadic<Value>>(temp)?;
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
    function! {
        lua fn temp.name(lua) {
            Ok("name")
        }
    }

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

#[derive(Debug, Clone, Copy, Typed, UserData, Deserialize)]
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
            .register_enum::<SystemColor>()?
            .register_enum::<Color>()?
            .register::<Example>()
            .value_with::<Example, _>("example", ["Example module"])
            .function_with::<String, (), _>("greet", (), ["Greet the name that was passed in"])
            .function_with::<Color, (), _>("printColor", (), ["Print a color and it's value"])
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

--- Example module
--- @type Example
example = nil

--- Greet the name that was passed in
--- @param param0 string
function greet(param0) end

--- Print a color and it's value
--- @param param0 Color
function printColor(param0) end
```
