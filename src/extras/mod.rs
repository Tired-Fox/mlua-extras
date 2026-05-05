use std::path::Path;

use mlua::{AnyUserData, FromLua, FromLuaMulti, IntoLua, IntoLuaMulti, Lua, Table, UserDataFields};

mod module;

pub use module::{ExtendModule, LuaModule, Module, ModuleBuilder, ModuleFields, ModuleMethods};

use crate::MaybeSend;

/// Adds quality of life helper methods to the [`Lua`] type
///
/// Helpers:
/// - [`path`](https://www.lua.org/manual/5.1/manual.html#pdf-package.path) and [`cpath`](https://www.lua.org/manual/5.1/manual.html#pdf-package.cpath) manipulation
/// - Shorthand for `lua.globals().set` that include adding any value and adding rust functions
///     skipping [`create_function`][mlua::Lua::create_function]
pub trait LuaExtras {
    /// Get the `package.path` value
    ///
    /// This is the value used by the lua engine to resolve `require` calls on `lua` files.
    /// see:
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.path>
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath>
    fn path(&self) -> mlua::Result<String>;

    /// Get the `package.cpath` value
    ///
    /// This is the value used by the lua engine to resolve `require` calls on `lib` files.
    /// see:
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.cpath>
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath>
    fn cpath(&self) -> mlua::Result<String>;

    /// Prepend a path tothe `package.path` value
    ///
    /// This is the value used by the lua engine to resolve `require` calls.
    /// see:
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.path>
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath>
    fn prepend_path<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()>;

    /// Prepend paths to the `package.path` value
    ///
    /// This is the value used by the lua engine to resolve `require` calls.
    /// see:
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.path>
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath>
    fn prepend_paths<S: AsRef<Path>>(&self, paths: impl IntoIterator<Item = S>)
    -> mlua::Result<()>;

    /// Append a path tothe `package.path` value
    ///
    /// This is the value used by the lua engine to resolve `require` calls.
    /// see:
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.path>
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath>
    fn append_path<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()>;

    /// Append paths to the `package.path` value
    ///
    /// This is the value used by the lua engine to resolve `require` calls.
    /// see:
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.path>
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath>
    fn append_paths<S: AsRef<Path>>(&self, paths: impl IntoIterator<Item = S>) -> mlua::Result<()>;

    /// Set the `package.path` value
    ///
    /// This is the value used by the lua engine to resolve `require` calls.
    /// see:
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.path>
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath>
    fn set_path<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()>;

    /// Set the `package.path` values
    ///
    /// This is the value used by the lua engine to resolve `require` calls.
    /// see:
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.path>
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath>
    fn set_paths<S: AsRef<Path>>(&self, paths: impl IntoIterator<Item = S>) -> mlua::Result<()>;

    /// Prepend a path tothe `package.cpath` value
    ///
    /// This is the value used by the lua engine to resolve `require` calls.
    /// see:
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.cpath>
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath>
    fn prepend_cpath<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()>;

    /// Prepend paths to the `package.cpath` value
    ///
    /// This is the value used by the lua engine to resolve `require` calls.
    /// see:
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.cpath>
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath>
    fn prepend_cpaths<S: AsRef<Path>>(
        &self,
        paths: impl IntoIterator<Item = S>,
    ) -> mlua::Result<()>;

    /// Append a path to the `package.cpath` value
    ///
    /// This is the value used by the lua engine to resolve `require` calls.
    /// see:
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.cpath>
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath>
    fn append_cpath<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()>;

    /// Append paths to the `package.cpath` value
    ///
    /// This is the value used by the lua engine to resolve `require` calls.
    /// see:
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.cpath>
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath>
    fn append_cpaths<S: AsRef<Path>>(&self, paths: impl IntoIterator<Item = S>)
    -> mlua::Result<()>;

    /// Set the `package.cpath` value
    ///
    /// This is the value used by the lua engine to resolve `require` calls.
    /// see:
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.cpath>
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath>
    fn set_cpath<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()>;

    /// Set the `package.cpath` values
    ///
    /// This is the value used by the lua engine to resolve `require` calls.
    /// see:
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.cpath>
    ///   - <https://www.lua.org/manual/5.4/manual.html#pdf-package.searchpath>
    fn set_cpaths<S: AsRef<Path>>(&self, paths: impl IntoIterator<Item = S>) -> mlua::Result<()>;

    /// Set a global variable
    fn set_global<K, V>(&self, key: K, value: V) -> mlua::Result<()>
    where
        K: IntoLua,
        V: IntoLua;

    fn set_global_function<K, A, R, F>(&self, key: K, value: F) -> mlua::Result<()>
    where
        K: IntoLua,
        A: FromLuaMulti,
        R: IntoLuaMulti,
        F: Fn(&Lua, A) -> mlua::Result<R> + Send + 'static;
}

impl LuaExtras for Lua {
    fn set_global<K, V>(&self, key: K, value: V) -> mlua::Result<()>
    where
        K: IntoLua,
        V: IntoLua,
    {
        self.globals().set(key, value)
    }

    fn set_global_function<K, A, R, F>(&self, key: K, value: F) -> mlua::Result<()>
    where
        K: IntoLua,
        A: FromLuaMulti,
        R: IntoLuaMulti,
        F: Fn(&Lua, A) -> mlua::Result<R> + Send + 'static,
    {
        self.globals().set(key, self.create_function(value)?)
    }

    fn path(&self) -> mlua::Result<String> {
        self.globals()
            .get::<Table>("package")?
            .get::<String>("path")
    }

    fn cpath(&self) -> mlua::Result<String> {
        self.globals()
            .get::<Table>("package")?
            .get::<String>("cpath")
    }

    fn set_path<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()> {
        self.globals()
            .get::<Table>("package")
            .unwrap()
            .set("path", path.as_ref().display().to_string())
    }

    fn set_paths<S: AsRef<Path>>(&self, paths: impl IntoIterator<Item = S>) -> mlua::Result<()> {
        self.globals().get::<Table>("package").unwrap().set(
            "path",
            paths
                .into_iter()
                .map(|s| s.as_ref().display().to_string())
                .collect::<Vec<_>>()
                .join(";"),
        )
    }

    fn prepend_path<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()> {
        let lua_path = match self.path()?.trim() {
            "" => path.as_ref().display().to_string(),
            other => format!("{};{other}", path.as_ref().display()),
        };
        self.globals()
            .get::<Table>("package")?
            .set("path", lua_path)
    }

    fn prepend_paths<S: AsRef<Path>>(
        &self,
        paths: impl IntoIterator<Item = S>,
    ) -> mlua::Result<()> {
        let new = paths
            .into_iter()
            .map(|v| v.as_ref().display().to_string())
            .collect::<Vec<_>>()
            .join(";");
        let lua_path = match self.path()?.trim() {
            "" => new,
            other => format!("{new};{other}"),
        };
        self.globals()
            .get::<Table>("package")?
            .set("path", lua_path)
    }

    fn append_path<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()> {
        let lua_path = match self.path()?.trim() {
            "" => path.as_ref().display().to_string(),
            other => format!("{other};{}", path.as_ref().display()),
        };
        self.globals()
            .get::<Table>("package")?
            .set("path", lua_path)
    }

    fn append_paths<S: AsRef<Path>>(&self, paths: impl IntoIterator<Item = S>) -> mlua::Result<()> {
        let new = paths
            .into_iter()
            .map(|v| v.as_ref().display().to_string())
            .collect::<Vec<_>>()
            .join(";");
        let lua_path = match self.path()?.trim() {
            "" => new,
            other => format!("{other};{new}"),
        };
        self.globals()
            .get::<Table>("package")?
            .set("path", lua_path)
    }

    fn set_cpath<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()> {
        self.globals()
            .get::<Table>("package")
            .unwrap()
            .set("cpath", path.as_ref().display().to_string())
    }

    fn set_cpaths<S: AsRef<Path>>(&self, paths: impl IntoIterator<Item = S>) -> mlua::Result<()> {
        self.globals().get::<Table>("package").unwrap().set(
            "cpath",
            paths
                .into_iter()
                .map(|s| s.as_ref().display().to_string())
                .collect::<Vec<_>>()
                .join(";"),
        )
    }

    fn prepend_cpath<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()> {
        let lua_path = match self.path()?.trim() {
            "" => path.as_ref().display().to_string(),
            other => format!("{};{other}", path.as_ref().display()),
        };
        self.globals()
            .get::<Table>("package")?
            .set("cpath", lua_path)
    }

    fn prepend_cpaths<S: AsRef<Path>>(
        &self,
        paths: impl IntoIterator<Item = S>,
    ) -> mlua::Result<()> {
        let new = paths
            .into_iter()
            .map(|v| v.as_ref().display().to_string())
            .collect::<Vec<_>>()
            .join(";");
        let lua_path = match self.path()?.trim() {
            "" => new,
            other => format!("{new};{other}"),
        };
        self.globals()
            .get::<Table>("package")?
            .set("cpath", lua_path)
    }

    fn append_cpath<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()> {
        let lua_path = match self.cpath()?.trim() {
            "" => path.as_ref().display().to_string(),
            other => format!("{other};{}", path.as_ref().display()),
        };
        self.globals()
            .get::<Table>("package")?
            .set("cpath", lua_path)
    }

    fn append_cpaths<S: AsRef<Path>>(
        &self,
        paths: impl IntoIterator<Item = S>,
    ) -> mlua::Result<()> {
        let new = paths
            .into_iter()
            .map(|v| v.as_ref().display().to_string())
            .collect::<Vec<_>>()
            .join(";");
        let lua_path = match self.path()?.trim() {
            "" => new,
            other => format!("{other};{new}"),
        };
        self.globals()
            .get::<Table>("package")?
            .set("cpath", lua_path)
    }
}

/// Helper that combines some of the assignments of fields for UserData
pub trait UserDataGetSet<T> {
    /// Combination of [add_field_method_get](mlua::UserDataFields::add_field_method_get) and [add_field_method_set](mlua::UserDataFields::add_field_method_set)
    fn add_field_method_get_set<S, R, A, GET, SET>(&mut self, name: S, get: GET, set: SET)
    where
        S: Into<String>,
        R: IntoLua,
        A: FromLua,
        GET: 'static + MaybeSend + Fn(&Lua, &T) -> mlua::Result<R>,
        SET: 'static + MaybeSend + Fn(&Lua, &mut T, A) -> mlua::Result<()>;

    /// Typed version of [add_field_function_get](mlua::UserDataFields::add_field_function_get) and [add_field_function_set](mlua::UserDataFields::add_field_function_set) combined
    fn add_field_function_get_set<S, R, A, GET, SET>(&mut self, name: S, get: GET, set: SET)
    where
        S: Into<String>,
        R: IntoLua,
        A: FromLua,
        GET: 'static + MaybeSend + Fn(&Lua, AnyUserData) -> mlua::Result<R>,
        SET: 'static + MaybeSend + Fn(&Lua, AnyUserData, A) -> mlua::Result<()>;
}

impl<T, U: UserDataFields<T>> UserDataGetSet<T> for U {
    fn add_field_method_get_set<S, R, A, GET, SET>(&mut self, name: S, get: GET, set: SET)
    where
        S: Into<String>,
        R: IntoLua,
        A: FromLua,
        GET: Fn(&Lua, &T) -> mlua::Result<R> + MaybeSend + 'static,
        SET: Fn(&Lua, &mut T, A) -> mlua::Result<()> + MaybeSend + 'static,
    {
        let name: String = name.into();
        self.add_field_method_get(&name, get);
        self.add_field_method_set(name, set);
    }

    fn add_field_function_get_set<S, R, A, GET, SET>(&mut self, name: S, get: GET, set: SET)
    where
        S: Into<String>,
        R: IntoLua,
        A: FromLua,
        GET: Fn(&Lua, AnyUserData) -> mlua::Result<R> + MaybeSend + 'static,
        SET: Fn(&Lua, AnyUserData, A) -> mlua::Result<()> + MaybeSend + 'static,
    {
        let name: String = name.into();
        self.add_field_function_get(&name, get);
        self.add_field_function_set(name, set);
    }
}
