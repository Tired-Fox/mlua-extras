use std::{collections::VecDeque, path::PathBuf, sync::Arc};

use mlua::{ExternalResult, Lua, LuaOptions, OwnedThread, StdLib, Value};
use mlua_extras::{function, LuaExtras};

#[cfg(feature="uv")]
use libuv::WorkReq;
#[cfg(feature="uv")]
use parking_lot::{Mutex, RwLock};

#[cfg(feature="uv")]
fn run_loop(lua: &Lua) -> mlua_extras::Result<()> {
    let mut r#loop = libuv::Loop::default().into_lua_err()?;

    let handler_one = function! {
        #[lua]
        fn handler(lua) {
            std::thread::sleep(std::time::Duration::from_secs(2));
            println!("hello one");
            Ok(())
        }
    }?;

    let handler_two = function! {
        #[lua]
        fn handler(lua) {
            std::thread::sleep(std::time::Duration::from_secs(3));
            println!("hello two");
            Ok(())
        }
    }?;


    let workload = Arc::new(Mutex::new(VecDeque::<OwnedThread>::from([
        lua.create_thread(handler_one).into_lua_err()?.into_owned(),
        lua.create_thread(handler_two).into_lua_err()?.into_owned()
    ])));


    let run_thread = |wl: Arc<Mutex<VecDeque<OwnedThread>>>, index: usize| {
        move |_: WorkReq| {
            loop {
                let thread = wl.lock().pop_front();
                match thread {
                    None => break,
                    Some(thread) => loop {
                        match thread.resume::<_, Value>(()) {
                            Err(mlua::Error::CoroutineInactive) => break,
                            Err(error) => eprintln!("{error}"),
                            Ok(_) => {}
                        }
                    }
                }
            }
        }
    };

    r#loop.queue_work(run_thread(workload.clone(), 1), move |_, _| {}).into_lua_err()?;
    r#loop.queue_work(run_thread(workload.clone(), 2), move |_, _| {}).into_lua_err()?;
    r#loop.queue_work(run_thread(workload.clone(), 3), move |_, _| {}).into_lua_err()?;
    r#loop.queue_work(run_thread(workload.clone(), 4), move |_, _| {}).into_lua_err()?;

    r#loop.run(libuv::RunMode::Default).into_lua_err()?;
    Ok(())
}

#[tokio::main]
async fn main() -> mlua_extras::Result<()> {
    let lua = unsafe { Lua::unsafe_new_with(StdLib::ALL, LuaOptions::new()) };
    
    // Can prepend, append, or set (override) the paths for mlua
    lua.prepend_path(PathBuf::from("examples").join("?").join("init.lua"))?;
    lua.prepend_path(PathBuf::from("examples").join("?.lua"))?;

    #[cfg(feature="uv")]
    run_loop(&lua)?;

    lua.load(r#"require 'libuv'"#).eval::<mlua::Value>()?;

    Ok(())
}
