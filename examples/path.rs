use mlua_extras::{
    extras::LuaExtras,
    mlua::{self, Lua},
};
use std::path::PathBuf;

// Use the provided result to have the lua error be reported (printed)
// in a formatted style.
//
// This is because results returned from main are formatted with the `Debug`
// formatter.
//
// `mlua::Error` does not format/build the error with `Debug` format but only with `Display`
// format
fn main() -> mlua::Result<()> {
    let lua = Lua::new();

    // Can prepend, append, or set (override) the paths for mlua
    lua.prepend_path(PathBuf::from("examples").join("?").join("init.lua"))?;
    lua.prepend_path(PathBuf::from("examples").join("?.lua"))?;

    // Can prepend, append, or set (override) the cpaths for mlua
    lua.append_cpath(PathBuf::from("examples").join("?.dll"))?;
    lua.append_cpath(PathBuf::from("examples").join("?.lib"))?;

    // Set globals in with shorthand helpers
    lua.set_global("key", "value")?;
    lua.set_global_function("hello", |_lua, _: ()| {
        println!("Hello, world!");
        Ok(())
    })?;

    // long hand
    lua.globals().set("key", "value")?;
    lua.globals().set(
        "hello",
        lua.create_function(|_lua, _: ()| {
            println!("Hello, world!");
            Ok(())
        })?,
    )?;

    lua.load("require 'init'").eval::<mlua::Value>()?;

    Ok(())
}
