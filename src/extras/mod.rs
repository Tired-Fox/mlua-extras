use std::path::Path;

use mlua::{AnyUserData, FromLua, FromLuaMulti, IntoLua, IntoLuaMulti, Lua, Table, UserDataFields};

mod macros;
mod module;
mod require;

pub use module::{LuaModule, Module, ModuleBuilder, ModuleFields, ModuleMethods, ExtendModule};
pub use require::Require;

use crate::MaybeSend;

/// Adds quality of life helper methods to the [`Lua`] type
///
/// Helpers:
/// - [`path`](https://www.lua.org/manual/5.1/manual.html#pdf-package.path) and [`cpath`](https://www.lua.org/manual/5.1/manual.html#pdf-package.cpath) manipulation
/// - Shorthand for `lua.globals().set` that include adding any value and adding rust functions
///     skipping [`create_function`][mlua::Lua::create_function]
/// - A `require` method that is similar to the [`Require`] traits. Allows for `lua` style [`require`](https://www.lua.org/manual/5.1/manual.html#pdf-require)
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

    /// Fetch a nested lua value starting from lua's globals
    fn require<'lua, R: FromLua<'lua>>(&'lua self, path: impl AsRef<str>) -> mlua::Result<R>;
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
        self.globals()
            .get::<_, Table>("package")
            .unwrap()
            .set("path", path.as_ref().display().to_string())
    }

    fn set_paths<S: AsRef<Path>>(&self, paths: impl IntoIterator<Item = S>) -> mlua::Result<()> {
        self.globals().get::<_, Table>("package").unwrap().set(
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
            .get::<_, Table>("package")?
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
            .get::<_, Table>("package")?
            .set("path", lua_path)
    }

    fn append_path<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()> {
        let lua_path = match self.path()?.trim() {
            "" => path.as_ref().display().to_string(),
            other => format!("{other};{}", path.as_ref().display()),
        };
        self.globals()
            .get::<_, Table>("package")?
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
            .get::<_, Table>("package")?
            .set("path", lua_path)
    }

    fn set_cpath<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()> {
        self.globals()
            .get::<_, Table>("package")
            .unwrap()
            .set("cpath", path.as_ref().display().to_string())
    }

    fn set_cpaths<S: AsRef<Path>>(&self, paths: impl IntoIterator<Item = S>) -> mlua::Result<()> {
        self.globals().get::<_, Table>("package").unwrap().set(
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
            .get::<_, Table>("package")?
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
            .get::<_, Table>("package")?
            .set("cpath", lua_path)
    }

    fn append_cpath<S: AsRef<Path>>(&self, path: S) -> mlua::Result<()> {
        let lua_path = match self.path()?.trim() {
            "" => path.as_ref().display().to_string(),
            other => format!("{other};{}", path.as_ref().display()),
        };
        self.globals()
            .get::<_, Table>("package")?
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
            .get::<_, Table>("package")?
            .set("cpath", lua_path)
    }

    fn require<'lua, R: FromLua<'lua>>(&'lua self, path: impl AsRef<str>) -> mlua::Result<R> {
        let segments = path
            .as_ref()
            .split('.')
            .filter_map(|v| (!v.is_empty()).then_some(v.trim()))
            .collect::<Vec<_>>();

        let mut module = self.globals();
        if !segments.is_empty() {
            for seg in &segments[..segments.len() - 1] {
                module = module.get::<_, Table>(*seg)?;
            }
        }

        match segments.last() {
            Some(seg) => module.get::<_, R>(*seg),
            None => Err(mlua::Error::runtime(format!(
                "module not found: {:?}",
                path.as_ref()
            ))),
        }
    }
}

/// Helper that combines some of the assignments of fields for UserData
pub trait UserDataGetSet<'lua, T> {
    /// Combination of [add_field_method_get](mlua::UserDataFields::add_field_method_get) and [add_field_method_set](mlua::UserDataFields::add_field_method_set)
    fn add_field_method_get_set<S, R, A, GET, SET>(&mut self, name: &S, get: GET, set: SET)
    where
        S: AsRef<str> + ?Sized,
        R: IntoLua<'lua>,
        A: FromLua<'lua>,
        GET: 'static + MaybeSend + Fn(&'lua Lua, &T) -> mlua::Result<R>,
        SET: 'static + MaybeSend + Fn(&'lua Lua, &mut T, A) -> mlua::Result<()>;

    /// Typed version of [add_field_function_get](mlua::UserDataFields::add_field_function_get) and [add_field_function_set](mlua::UserDataFields::add_field_function_set) combined
    fn add_field_function_get_set<S, R, A, GET, SET>(&mut self, name: &S, get: GET, set: SET)
    where
        S: AsRef<str> + ?Sized,
        R: IntoLua<'lua>,
        A: FromLua<'lua>,
        GET: 'static + MaybeSend + Fn(&'lua Lua, AnyUserData<'lua>) -> mlua::Result<R>,
        SET: 'static + MaybeSend + Fn(&'lua Lua, AnyUserData<'lua>, A) -> mlua::Result<()>;
}

impl<'lua, T, U: UserDataFields<'lua, T>> UserDataGetSet<'lua, T> for U {
    fn add_field_method_get_set<S, R, A, GET, SET>(&mut self, name: &S, get: GET, set: SET)
    where
        S: AsRef<str> + ?Sized,
        R: IntoLua<'lua>,
        A: FromLua<'lua>,
        GET: 'static + MaybeSend + Fn(&'lua Lua, &T) -> mlua::Result<R>,
        SET: 'static + MaybeSend + Fn(&'lua Lua, &mut T, A) -> mlua::Result<()>,
    {
        self.add_field_method_get(name, get);
        self.add_field_method_set(name, set);
    }

    fn add_field_function_get_set<S, R, A, GET, SET>(&mut self, name: &S, get: GET, set: SET)
    where
        S: AsRef<str> + ?Sized,
        R: IntoLua<'lua>,
        A: FromLua<'lua>,
        GET: 'static + MaybeSend + Fn(&'lua Lua, AnyUserData<'lua>) -> mlua::Result<R>,
        SET: 'static + MaybeSend + Fn(&'lua Lua, AnyUserData<'lua>, A) -> mlua::Result<()>,
    {
        self.add_field_function_get(name, get);
        self.add_field_function_set(name, set);
    }
}
