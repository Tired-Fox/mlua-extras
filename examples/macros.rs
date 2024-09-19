use std::path::PathBuf;

use mlua::{Function, Lua, LuaOptions, StdLib};
use mlua_extras::{function, require::{Import, TableImport as _}, LuaExtras};

fn main() -> mlua_extras::Result<()> {
    let lua = unsafe { Lua::unsafe_new_with(StdLib::ALL, LuaOptions::new()) };
    
    lua.prepend_path(PathBuf::from("examples").join("?").join("init.lua"))?;
    lua.prepend_path(PathBuf::from("examples").join("?.lua"))?;

    let custom_function = function! {
        /// Some function documentation
        lua fn add(_lua: &mlua::Lua, a: usize, b: usize) {
            Ok(a + b)
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
    assert_eq!(temp.require::<Function>("add")?.call::<_, usize>((1, 2))?, 3);
    assert_eq!(temp.require::<Function>("nested.add")?.call::<_, usize>((1, 2))?, 3);

    // TODO: Move this out of the repo
    // Requires the following:
    // 1. `luarocks` with all of it's paths added to `LUA_PATH` and `LUA_CPATH`
    // 2. `lua5.4` installed and added to `PATH`. i.e. `<Location of lua root dir>/bin`. This is
    //    where `lua54.dll`/`lua54.lib` is located.
    // 3. The `luv` package from `luarocks` to be installed. Requires `cmake` and `mingw` on
    //    windows
    //
    // All these steps should be simple if you are used to using `luarocks`.
    lua.load(r#"require 'libuv'"#).eval::<mlua::Value>()?;

    Ok(())
}
