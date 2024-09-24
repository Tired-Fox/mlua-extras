use std::{any::type_name, marker::PhantomData};

use mlua::{FromLuaMulti, IntoLua, IntoLuaMulti, Lua, Table};

use crate::MaybeSend;

#[derive(Default)]
pub struct LuaModule<M>(PhantomData<M>);
impl<'lua, M: Module> IntoLua<'lua> for LuaModule<M> {
    fn into_lua(
        self,
        lua: &'lua mlua::prelude::Lua,
    ) -> mlua::prelude::LuaResult<mlua::prelude::LuaValue<'lua>> {
        let mut builder: ModuleBuilder<'lua> = ModuleBuilder {
            table: lua.create_table()?,
            lua,
            parents: Vec::new(),
        };

        M::add_fields(&mut builder)?;
        M::add_methods(&mut builder)?;

        Ok(mlua::Value::Table(builder.table))
    }
}

/// Extend the contents of a table (module) with the contents of a [`Module`]
///
/// Instead of creating a new table when initializing and calling [`Module`] it will use the
/// given parent as the source to add it's fields, functions, and methods to.
pub trait ExtendModule<'lua> {
    /// Extend the current content with the contents of a [`Module`]
    fn extend<M: Module>(&mut self, lua: &'lua Lua) -> mlua::Result<()>;
}

impl<'lua> ExtendModule<'lua> for Table<'lua> {
    fn extend<M: Module>(&mut self, lua: &'lua Lua) -> mlua::Result<()> {
        let mut builder: ModuleBuilder<'lua> = ModuleBuilder {
            table: self.clone(),
            lua,
            parents: Vec::new(),
        };

        M::add_fields(&mut builder)?;
        M::add_methods(&mut builder)?;

        Ok(())
    }
}

/// Sepecify a lua module (table) with fields and methods
pub trait Module: Sized {
    /// Add fields to the module
    #[allow(unused_variables)]
    fn add_fields<'lua, F: ModuleFields<'lua>>(fields: &mut F) -> mlua::Result<()> {
        Ok(())
    }
    
    /// Add methods/functions to the module
    #[allow(unused_variables)]
    fn add_methods<'lua, M: ModuleMethods<'lua>>(methods: &mut M) -> mlua::Result<()> {
        Ok(())
    }

    fn module() -> LuaModule<Self> {
        LuaModule(PhantomData)
    }
}

/// Add table fields for a module
pub trait ModuleFields<'lua> {
    /// Add a field to the module's table
    fn add_field<K, V>(&mut self, name: K, value: V) -> mlua::Result<()>
    where
        K: IntoLua<'lua>,
        V: IntoLua<'lua>;

    /// Add a field to the module's metatable
    fn add_meta_field<K, V>(&mut self, name: K, value: V) -> mlua::Result<()>
    where
        K: IntoLua<'lua>,
        V: IntoLua<'lua>;

    /// Add a nested module as a table in this module
    fn add_module<K, V>(&mut self, name: K) -> mlua::Result<()>
    where
        K: IntoLua<'lua>,
        V: Module;
}

/// Add table functions and methods for a module
pub trait ModuleMethods<'lua> {
    /// Add a function to this module's table
    fn add_function<K, F, A, R>(&mut self, name: K, function: F) -> mlua::Result<()>
    where
        K: IntoLua<'lua>,
        F: Fn(&'lua mlua::Lua, A) -> mlua::Result<R> + MaybeSend + 'static,
        A: FromLuaMulti<'lua>,
        R: IntoLuaMulti<'lua>;

    /// Add a function to this module's metatable
    fn add_meta_function<K, F, A, R>(&mut self, name: K, function: F) -> mlua::Result<()>
    where
        K: IntoLua<'lua>,
        F: Fn(&'lua mlua::Lua, A) -> mlua::Result<R> + MaybeSend + 'static,
        A: FromLuaMulti<'lua>,
        R: IntoLuaMulti<'lua>;

    /// Add a method to this module's table
    fn add_method<K, F, A, R>(&mut self, name: K, function: F) -> mlua::Result<()>
    where
        K: IntoLua<'lua>,
        F: Fn(&'lua mlua::Lua, mlua::Table<'_>, A) -> mlua::Result<R> + MaybeSend + 'static,
        A: FromLuaMulti<'lua>,
        R: IntoLuaMulti<'lua>;

    /// Add a method to this module's metatable
    fn add_meta_method<K, F, A, R>(&mut self, name: K, function: F) -> mlua::Result<()>
    where
        K: IntoLua<'lua>,
        F: Fn(&'lua mlua::Lua, mlua::Table<'_>, A) -> mlua::Result<R> + MaybeSend + 'static,
        A: FromLuaMulti<'lua>,
        R: IntoLuaMulti<'lua>;
}

/// Builder that construct a module based on the [`Module`] trait
pub struct ModuleBuilder<'lua> {
    lua: &'lua mlua::Lua,
    table: mlua::Table<'lua>,
    parents: Vec<&'static str>,
}

impl<'lua> ModuleFields<'lua> for ModuleBuilder<'lua> {
    fn add_field<K, V>(&mut self, name: K, value: V) -> mlua::Result<()>
    where
        K: IntoLua<'lua>,
        V: IntoLua<'lua>,
    {
        self.table.set(name, value)
    }

    fn add_meta_field<K, V>(&mut self, name: K, value: V) -> mlua::Result<()>
    where
        K: IntoLua<'lua>,
        V: IntoLua<'lua>,
    {
        let meta = match self.table.get_metatable() {
            Some(meta) => meta,
            None => {
                let meta = self.lua.create_table()?;
                self.table.set_metatable(Some(meta.clone()));
                meta
            }
        };

        meta.set(name, value)
    }

    fn add_module<K, V>(&mut self, name: K) -> mlua::Result<()>
    where
        K: IntoLua<'lua>,
        V: Module,
    {
        if self.parents.contains(&type_name::<V>()) {
            return Err(mlua::Error::runtime(format!(
                "infinite nested modules using: '{}'",
                type_name::<V>()
            )));
        }

        let mut builder: ModuleBuilder<'lua> = ModuleBuilder {
            table: self.lua.create_table()?,
            lua: self.lua,
            parents: self
                .parents
                .iter()
                .map(|v| *v)
                .chain([type_name::<V>()])
                .collect(),
        };

        V::add_fields(&mut builder)?;
        V::add_methods(&mut builder)?;

        self.table.set(name, builder.table)
    }
}

impl<'lua> ModuleMethods<'lua> for ModuleBuilder<'lua> {
    fn add_function<K, F, A, R>(&mut self, name: K, function: F) -> mlua::Result<()>
    where
        K: IntoLua<'lua>,
        F: Fn(&'lua mlua::Lua, A) -> mlua::Result<R> + MaybeSend + 'static,
        A: FromLuaMulti<'lua>,
        R: IntoLuaMulti<'lua>,
    {
        self.table.set(name, self.lua.create_function(function)?)
    }

    fn add_meta_function<K, F, A, R>(&mut self, name: K, function: F) -> mlua::Result<()>
    where
        K: IntoLua<'lua>,
        F: Fn(&'lua mlua::Lua, A) -> mlua::Result<R> + MaybeSend + 'static,
        A: FromLuaMulti<'lua>,
        R: IntoLuaMulti<'lua>,
    {
        let meta = match self.table.get_metatable() {
            Some(meta) => meta,
            None => {
                let meta = self.lua.create_table()?;
                self.table.set_metatable(Some(meta.clone()));
                meta
            }
        };

        meta.set(name, self.lua.create_function(function)?)
    }

    fn add_method<K, F, A, R>(&mut self, name: K, function: F) -> mlua::Result<()>
    where
        K: IntoLua<'lua>,
        F: Fn(&'lua mlua::Lua, mlua::Table<'_>, A) -> mlua::Result<R> + MaybeSend + 'static,
        A: FromLuaMulti<'lua>,
        R: IntoLuaMulti<'lua>,
    {
        self.table.set(
            name,
            self.lua
                .create_function(move |lua, args: mlua::MultiValue| {
                    let this = mlua::Table::from_lua_multi(args.clone(), lua)?;
                    let rest = A::from_lua_multi(args, lua)?;
                    function(lua, this, rest)
                })?,
        )
    }

    fn add_meta_method<K, F, A, R>(&mut self, name: K, function: F) -> mlua::Result<()>
    where
        K: IntoLua<'lua>,
        F: Fn(&'lua mlua::Lua, mlua::Table<'_>, A) -> mlua::Result<R> + MaybeSend + 'static,
        A: FromLuaMulti<'lua>,
        R: IntoLuaMulti<'lua>,
    {
        let meta = match self.table.get_metatable() {
            Some(meta) => meta,
            None => {
                let meta = self.lua.create_table()?;
                self.table.set_metatable(Some(meta.clone()));
                meta
            }
        };

        meta.set(
            name,
            self.lua
                .create_function(move |lua, args: mlua::MultiValue| {
                    let this = mlua::Table::from_lua_multi(args.clone(), lua)?;
                    let rest = A::from_lua_multi(args, lua)?;
                    function(lua, this, rest)
                })?,
        )
    }
}
