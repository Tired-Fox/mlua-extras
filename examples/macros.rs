use std::path::PathBuf;

use mlua_extras::{
    mlua::{self, Function, Lua, LuaOptions, StdLib, Table, Value},
    extras::{LuaExtras, Require},
    function,
};

fn main() -> mlua::Result<()> {
    let lua = unsafe { Lua::unsafe_new_with(StdLib::ALL, LuaOptions::new()) };

    lua.prepend_path(PathBuf::from("examples").join("?").join("init.lua"))?;
    lua.prepend_path(PathBuf::from("examples").join("?.lua"))?;

    let custom_function = function! {
        /// Some function documentation
        lua fn add(_lua: &mlua::Lua, a: usize, b: usize) {
            Ok(a + b)
        }
    }?;

    // Extend an existing table with a function
    let table = lua.globals().get::<_, Table>("table")?;
    function! {
        lua fn table.keys(_lua, this: Table) {
            this.pairs::<Value, Value>()
                .map(|pair| {
                    let pair = pair?;
                    Ok(pair.0)
                })
                .collect::<mlua::Result<Vec<_>>>()
        }
    }?;
    function! {
         lua fn table.values(_lua, this: Table) {
            this.pairs::<Value, Value>()
                .map(|pair| {
                    let pair = pair?;
                    Ok(pair.1)
                })
            .collect::<mlua::Result<Vec<_>>>()
        }
    }?;

    let temp = lua.create_table()?;
    temp.set("nested", lua.create_table()?)?;

    // Add a function to temp
    function! {
        lua fn temp.add(_lua, a: usize, b: usize) {
            Ok(a + b)
        }
    }?;

    // Add a function to temp.nested
    function! {
        lua fn temp::nested.add(_lua, a: usize, b: usize) {
            Ok(a + b)
        }
    }?;

    assert_eq!(custom_function.call::<_, usize>((1, 2))?, 3);
    assert_eq!(
        temp.require::<Function>("add")?.call::<_, usize>((1, 2))?,
        3
    );
    assert_eq!(
        temp.require::<Function>("nested.add")?
            .call::<_, usize>((1, 2))?,
        3
    );

    Ok(())
}
