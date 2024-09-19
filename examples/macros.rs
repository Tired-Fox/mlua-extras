use std::path::PathBuf;

use mlua::{Function, Lua, LuaOptions, StdLib};
use mlua_extras::{function, require::TableImport as _, LuaExtras};

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

    Ok(())
}
