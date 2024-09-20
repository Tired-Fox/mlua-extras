use std::path::Path;

use mlua::{FromLuaMulti, IntoLua, IntoLuaMulti, Lua, Table};

pub mod require;
pub mod typed;
mod error;
mod macros;

pub use mlua_extras_derive::{UserData, Typed};

pub use error::{Report, Result};

// Quality of Life
//
// Utilities for manipulating the path, setting global variables, etc.
pub trait LuaExtras {
    /// Get the `package.path` value
    ///
    /// This is the value used by the lua engine to resolve `require` calls on `lua` files.
    /// see: 
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.path
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath
    fn path(&self) -> mlua::Result<String>;
    
    /// Get the `package.cpath` value
    ///
    /// This is the value used by the lua engine to resolve `require` calls on `lib` files.
    /// see: 
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.cpath
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath
    fn cpath(&self) -> mlua::Result<String>;

    /// Prepend a path tothe `package.path` value
    ///
    /// This is the value used by the lua engine to resolve `require` calls.
    /// see: 
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.path
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath
    fn prepend_path<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()>;
    
    /// Prepend paths to the `package.path` value
    ///
    /// This is the value used by the lua engine to resolve `require` calls.
    /// see: 
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.path
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath
    fn prepend_paths<S: AsRef<Path>>(&self, paths: impl IntoIterator<Item=S>) -> mlua::Result<()>;

    /// Append a path tothe `package.path` value
    ///
    /// This is the value used by the lua engine to resolve `require` calls.
    /// see: 
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.path
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath
    fn append_path<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()>;

    /// Append paths to the `package.path` value
    ///
    /// This is the value used by the lua engine to resolve `require` calls.
    /// see: 
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.path
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath
    fn append_paths<S: AsRef<Path>>(&self, paths: impl IntoIterator<Item=S>) -> mlua::Result<()>;

    /// Set the `package.path` value
    ///
    /// This is the value used by the lua engine to resolve `require` calls.
    /// see: 
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.path
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath
    fn set_path<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()>;

    /// Set the `package.path` values
    ///
    /// This is the value used by the lua engine to resolve `require` calls.
    /// see: 
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.path
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath
    fn set_paths<S: AsRef<Path>>(&self, paths: impl IntoIterator<Item=S>) -> mlua::Result<()>;

    /// Prepend a path tothe `package.cpath` value
    ///
    /// This is the value used by the lua engine to resolve `require` calls.
    /// see: 
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.cpath
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath
    fn prepend_cpath<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()>;
    
    /// Prepend paths to the `package.cpath` value
    ///
    /// This is the value used by the lua engine to resolve `require` calls.
    /// see: 
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.cpath
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath
    fn prepend_cpaths<S: AsRef<Path>>(&self, paths: impl IntoIterator<Item=S>) -> mlua::Result<()>;

    /// Append a path to the `package.cpath` value
    ///
    /// This is the value used by the lua engine to resolve `require` calls.
    /// see: 
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.cpath
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath
    fn append_cpath<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()>;

    /// Append paths to the `package.cpath` value
    ///
    /// This is the value used by the lua engine to resolve `require` calls.
    /// see: 
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.cpath
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath
    fn append_cpaths<S: AsRef<Path>>(&self, paths: impl IntoIterator<Item=S>) -> mlua::Result<()>;

    /// Set the `package.cpath` value
    ///
    /// This is the value used by the lua engine to resolve `require` calls.
    /// see: 
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.cpath
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath
    fn set_cpath<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()>;

    /// Set the `package.cpath` values
    ///
    /// This is the value used by the lua engine to resolve `require` calls.
    /// see: 
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.cpath
    ///   - https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath
    fn set_cpaths<S: AsRef<Path>>(&self, paths: impl IntoIterator<Item=S>) -> mlua::Result<()>;

    /// Set a global variable
    fn set_global<'lua, K, V>(&'lua self, key: K, value: V) -> mlua::Result<()>
    where
        K: IntoLua<'lua>,
        V: IntoLua<'lua>;

    fn set_global_function<'lua, K, A, R, F>(&'lua self, key: K, value: F) -> mlua::Result<()>
    where
        K: IntoLua<'lua>,
        A: FromLuaMulti<'lua>,
        R: IntoLuaMulti<'lua>,
        F: Fn(&'lua Lua, A) -> mlua::Result<R> + Send + 'static;
}

impl LuaExtras for Lua {
    fn set_global<'lua, K, V>(&'lua self, key: K, value: V) -> mlua::Result<()>
    where
        K: IntoLua<'lua>,
        V: IntoLua<'lua>,
    {
        self.globals().set(key, value)     
    }

    fn set_global_function<'lua, K, A, R, F>(&'lua self, key: K, value: F) -> mlua::Result<()>
    where
        K: IntoLua<'lua>,
        A: FromLuaMulti<'lua>,
        R: IntoLuaMulti<'lua>,
        F: Fn(&'lua Lua, A) -> mlua::Result<R> + Send + 'static,
    {
        self.globals().set(key, self.create_function(value)?)
    }

    fn path(&self) -> mlua::Result<String> {
        self.globals()
            .get::<_, Table>("package")?
            .get::<_, String>("path")
    }

    fn cpath(&self) -> mlua::Result<String> {
        self.globals()
            .get::<_, Table>("package")?
            .get::<_, String>("cpath")
    }

    fn set_path<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()> {
        self.globals().get::<_, Table>("package").unwrap().set("path", path.as_ref().display().to_string())
    }

    fn set_paths<S: AsRef<Path>>(&self, paths: impl IntoIterator<Item=S>) -> mlua::Result<()> {
        self.globals().get::<_, Table>("package")
            .unwrap()
            .set(
                "path",
                paths
                    .into_iter()
                    .map(|s| s.as_ref().display().to_string())
                    .collect::<Vec<_>>()
                    .join(";")
            )
    }

    fn prepend_path<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()> {
        let lua_path = match self.path()?.trim() {
            "" => path.as_ref().display().to_string(),
            other => format!("{};{other}", path.as_ref().display())
        };
        self.globals()
            .get::<_, Table>("package")?
            .set("path", lua_path)
    }

    fn prepend_paths<S: AsRef<Path>>(&self, paths: impl IntoIterator<Item=S>) -> mlua::Result<()> {
        let new = paths.into_iter().map(|v| v.as_ref().display().to_string()).collect::<Vec<_>>().join(";");
        let lua_path = match self.path()?.trim() {
            "" => new,
            other => format!("{new};{other}"),
        };
        self.globals()
            .get::<_, Table>("package")?
            .set("path", lua_path)
    }

    fn append_path<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()> {
        let lua_path = match self.path()?.trim() {
            "" => path.as_ref().display().to_string(),
            other => format!("{other};{}", path.as_ref().display())
        };
        self.globals()
            .get::<_, Table>("package")?
            .set("path", lua_path)
    }

    fn append_paths<S: AsRef<Path>>(&self, paths: impl IntoIterator<Item=S>) -> mlua::Result<()> {
        let new = paths.into_iter().map(|v| v.as_ref().display().to_string()).collect::<Vec<_>>().join(";");
        let lua_path = match self.path()?.trim() {
            "" => new,
            other => format!("{other};{new}"),
        };
        self.globals()
            .get::<_, Table>("package")?
            .set("path", lua_path)
    }

    fn set_cpath<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()> {
        self.globals().get::<_, Table>("package").unwrap().set("cpath", path.as_ref().display().to_string())
    }

    fn set_cpaths<S: AsRef<Path>>(&self, paths: impl IntoIterator<Item=S>) -> mlua::Result<()> {
        self.globals().get::<_, Table>("package")
            .unwrap()
            .set(
                "cpath",
                paths
                    .into_iter()
                    .map(|s| s.as_ref().display().to_string())
                    .collect::<Vec<_>>()
                    .join(";")
            )
    }

    fn prepend_cpath<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()> {
        let lua_path = match self.path()?.trim() {
            "" => path.as_ref().display().to_string(),
            other => format!("{};{other}", path.as_ref().display())
        };
        self.globals()
            .get::<_, Table>("package")?
            .set("cpath", lua_path)
    }

    fn prepend_cpaths<S: AsRef<Path>>(&self, paths: impl IntoIterator<Item=S>) -> mlua::Result<()> {
        let new = paths.into_iter().map(|v| v.as_ref().display().to_string()).collect::<Vec<_>>().join(";");
        let lua_path = match self.path()?.trim() {
            "" => new,
            other => format!("{new};{other}"),
        };
        self.globals()
            .get::<_, Table>("package")?
            .set("cpath", lua_path)
    }

    fn append_cpath<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()> {
        let lua_path = match self.path()?.trim() {
            "" => path.as_ref().display().to_string(),
            other => format!("{other};{}", path.as_ref().display())
        };
        self.globals()
            .get::<_, Table>("package")?
            .set("cpath", lua_path)
    }

    fn append_cpaths<S: AsRef<Path>>(&self, paths: impl IntoIterator<Item=S>) -> mlua::Result<()> {
        let new = paths.into_iter().map(|v| v.as_ref().display().to_string()).collect::<Vec<_>>().join(";");
        let lua_path = match self.path()?.trim() {
            "" => new,
            other => format!("{other};{new}"),
        };
        self.globals()
            .get::<_, Table>("package")?
            .set("cpath", lua_path)
    }
}
