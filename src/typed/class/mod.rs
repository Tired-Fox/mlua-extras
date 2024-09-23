use mlua::{AnyUserData, FromLua, FromLuaMulti, IntoLua, IntoLuaMulti, Lua, MetaMethod};

use crate::MaybeSend;

use super::{generator::FunctionBuilder, Typed, TypedMultiValue};

mod wrapped;
mod standard;

pub use wrapped::WrappedBuilder;
pub use standard::TypedClassBuilder;

/// Typed variant of [`UserData`]
pub trait TypedUserData: Sized {
    /// Add documentation to the type itself
    #[allow(unused_variables)]
    fn add_documentation<F: TypedDataDocumentation<Self>>(docs: &mut F) {}

    ///same as [UserData::add_methods].
    ///Refer to its documentation on how to use it.
    ///
    ///only difference is that it takes a [TypedDataMethods],
    ///which is the typed version of [UserDataMethods]
    #[allow(unused_variables)]
    fn add_methods<'lua, T: TypedDataMethods<'lua, Self>>(methods: &mut T) {}

    /// same as [UserData::add_fields].
    /// Refer to its documentation on how to use it.
    ///
    /// only difference is that it takes a [TypedDataFields],
    /// which is the typed version of [UserDataFields]
    #[allow(unused_variables)]
    fn add_fields<'lua, F: TypedDataFields<'lua, Self>>(fields: &mut F) {}
}

/// Used inside of [`TypedUserData`] to add doc comments to the userdata type itself
pub trait TypedDataDocumentation<T: TypedUserData> {
    fn add(&mut self, doc: &str) -> &mut Self;
}

/// Typed variant of [`UserDataFields`]
pub trait TypedDataMethods<'lua, T> {
    /// Exposes a method to lua
    fn add_method<S, A, R, M>(&mut self, name: &S, method: M)
    where
        S: ?Sized + AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        M: 'static + MaybeSend + Fn(&'lua Lua, &T, A) -> mlua::Result<R>;

    /// Exposes a method to lua
    ///
    /// Pass an additional callback that allows for param names, param doc comments, and return doc
    /// comments to be specified.
    fn add_method_with<S, A, R, M, G>(&mut self, name: &S, method: M, generator: G)
    where
        S: ?Sized + AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        M: 'static + MaybeSend + Fn(&'lua Lua, &T, A) -> mlua::Result<R>,
        G: Fn(&mut FunctionBuilder<A, R>);

    /// Exposes a method to lua that has a mutable reference to Self
    fn add_method_mut<S, A, R, M>(&mut self, name: &S, method: M)
    where
        S: ?Sized + AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        M: 'static + MaybeSend + FnMut(&'lua Lua, &mut T, A) -> mlua::Result<R>;

    /// Exposes a method to lua that has a mutable reference to Self
    ///
    /// Pass an additional callback that allows for param names, param doc comments, and return doc
    /// comments to be specified.
    fn add_method_mut_with<S, A, R, M, G>(&mut self, name: &S, method: M, generator: G)
    where
        S: ?Sized + AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        M: 'static + MaybeSend + FnMut(&'lua Lua, &mut T, A) -> mlua::Result<R>,
        G: Fn(&mut FunctionBuilder<A, R>);

    #[cfg(feature = "async")]
    ///exposes an async method to lua
    fn add_async_method<'s, S: ?Sized + AsRef<str>, A, R, M, MR>(&mut self, name: &S, method: M)
    where
        'lua: 's,
        T: 'static,
        M: Fn(&'lua Lua, &'s T, A) -> MR + MaybeSend + 'static,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        MR: std::future::Future<Output = mlua::Result<R>> + 's,
        R: IntoLuaMulti<'lua> + TypedMultiValue;

    #[cfg(feature = "async")]
    ///exposes an async method to lua
    ///
    /// Pass an additional callback that allows for param names, param doc comments, and return doc
    /// comments to be specified.
    fn add_async_method_with<'s, S: ?Sized + AsRef<str>, A, R, M, MR, G>(&mut self, name: &S, method: M, generator: G)
    where
        'lua: 's,
        T: 'static,
        M: Fn(&'lua Lua, &'s T, A) -> MR + MaybeSend + 'static,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        MR: std::future::Future<Output = mlua::Result<R>> + 's,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        G: Fn(&mut FunctionBuilder<A, R>);

    #[cfg(feature = "async")]
    ///exposes an async method to lua
    fn add_async_method_mut<'s, S: ?Sized + AsRef<str>, A, R, M, MR>(&mut self, name: &S, method: M)
    where
        'lua: 's,
        T: 'static,
        M: Fn(&'lua Lua, &'s mut T, A) -> MR + MaybeSend + 'static,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        MR: std::future::Future<Output = mlua::Result<R>> + 's,
        R: IntoLuaMulti<'lua> + TypedMultiValue;

    #[cfg(feature = "async")]
    ///exposes an async method to lua
    ///
    /// Pass an additional callback that allows for param names, param doc comments, and return doc
    /// comments to be specified.
    fn add_async_method_mut_with<'s, S: ?Sized + AsRef<str>, A, R, M, MR, G>(&mut self, name: &S, method: M, generator: G)
    where
        'lua: 's,
        T: 'static,
        M: Fn(&'lua Lua, &'s mut T, A) -> MR + MaybeSend + 'static,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        MR: std::future::Future<Output = mlua::Result<R>> + 's,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        G: Fn(&mut FunctionBuilder<A, R>);

    ///Exposes a function to lua (its a method that does not take Self)
    fn add_function<S, A, R, F>(&mut self, name: &S, function: F)
    where
        S: ?Sized + AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + Fn(&'lua Lua, A) -> mlua::Result<R>;

    ///Exposes a function to lua (its a method that does not take Self)
    ///
    /// Pass an additional callback that allows for param names, param doc comments, and return doc
    /// comments to be specified.
    fn add_function_with<S, A, R, F, G>(&mut self, name: &S, function: F, generator: G)
    where
        S: ?Sized + AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + Fn(&'lua Lua, A) -> mlua::Result<R>,
        G: Fn(&mut FunctionBuilder<A, R>);

    ///Exposes a mutable function to lua
    fn add_function_mut<S, A, R, F>(&mut self, name: &S, function: F)
    where
        S: ?Sized + AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + FnMut(&'lua Lua, A) -> mlua::Result<R>;

    ///Exposes a mutable function to lua
    ///
    /// Pass an additional callback that allows for param names, param doc comments, and return doc
    /// comments to be specified.
    fn add_function_mut_with<S, A, R, F, G>(&mut self, name: &S, function: F, generator: G)
    where
        S: ?Sized + AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + FnMut(&'lua Lua, A) -> mlua::Result<R>,
        G: Fn(&mut FunctionBuilder<A, R>);

    #[cfg(feature = "async")]
    ///exposes an async function to lua
    fn add_async_function<S: ?Sized, A, R, F, FR>(&mut self, name: &S, function: F)
    where
        S: AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + Fn(&'lua Lua, A) -> FR,
        FR: 'lua + std::future::Future<Output = mlua::Result<R>>;

    #[cfg(feature = "async")]
    ///exposes an async function to lua
    ///
    /// Pass an additional callback that allows for param names, param doc comments, and return doc
    /// comments to be specified.
    fn add_async_function_with<S: ?Sized, A, R, F, FR, G>(&mut self, name: &S, function: F, generator: G)
    where
        S: AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + Fn(&'lua Lua, A) -> FR,
        FR: 'lua + std::future::Future<Output = mlua::Result<R>>,
        G: Fn(&mut FunctionBuilder<A, R>);

    ///Exposes a meta method to lua [http://lua-users.org/wiki/MetatableEvents](http://lua-users.org/wiki/MetatableEvents)
    fn add_meta_method<A, R, M>(&mut self, meta: MetaMethod, method: M)
    where
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        M: 'static + MaybeSend + Fn(&'lua Lua, &T, A) -> mlua::Result<R>;

    ///Exposes a meta method to lua [http://lua-users.org/wiki/MetatableEvents](http://lua-users.org/wiki/MetatableEvents)
    ///
    /// Pass an additional callback that allows for param names, param doc comments, and return doc
    /// comments to be specified.
    fn add_meta_method_with<A, R, M, G>(&mut self, meta: MetaMethod, method: M, generator: G)
    where
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        M: 'static + MaybeSend + Fn(&'lua Lua, &T, A) -> mlua::Result<R>,
        G: Fn(&mut FunctionBuilder<A, R>);

    ///Exposes a meta and mutable method to lua [http://lua-users.org/wiki/MetatableEvents](http://lua-users.org/wiki/MetatableEvents)
    fn add_meta_method_mut<A, R, M>(&mut self, meta: MetaMethod, method: M)
    where
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        M: 'static + MaybeSend + FnMut(&'lua Lua, &mut T, A) -> mlua::Result<R>;

    ///Exposes a meta and mutable method to lua [http://lua-users.org/wiki/MetatableEvents](http://lua-users.org/wiki/MetatableEvents)
    ///
    /// Pass an additional callback that allows for param names, param doc comments, and return doc
    /// comments to be specified.
    fn add_meta_method_mut_with<A, R, M, G>(&mut self, meta: MetaMethod, method: M, generator: G)
    where
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        M: 'static + MaybeSend + FnMut(&'lua Lua, &mut T, A) -> mlua::Result<R>,
        G: Fn(&mut FunctionBuilder<A, R>);

    ///Exposes a meta function to lua [http://lua-users.org/wiki/MetatableEvents](http://lua-users.org/wiki/MetatableEvents)
    fn add_meta_function<A, R, F>(&mut self, meta: MetaMethod, function: F)
    where
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + Fn(&'lua Lua, A) -> mlua::Result<R>;

    ///Exposes a meta function to lua [http://lua-users.org/wiki/MetatableEvents](http://lua-users.org/wiki/MetatableEvents)
    ///
    /// Pass an additional callback that allows for param names, param doc comments, and return doc
    /// comments to be specified.
    fn add_meta_function_with<A, R, F, G>(&mut self, meta: MetaMethod, function: F, generator: G)
    where
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + Fn(&'lua Lua, A) -> mlua::Result<R>,
        G: Fn(&mut FunctionBuilder<A, R>);

    ///Exposes a meta and mutable function to lua [http://lua-users.org/wiki/MetatableEvents](http://lua-users.org/wiki/MetatableEvents)
    fn add_meta_function_mut<A, R, F>(&mut self, meta: MetaMethod, function: F)
    where
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + FnMut(&'lua Lua, A) -> mlua::Result<R>;

    ///Exposes a meta and mutable function to lua [http://lua-users.org/wiki/MetatableEvents](http://lua-users.org/wiki/MetatableEvents)
    fn add_meta_function_mut_with<A, R, F, G>(&mut self, meta: MetaMethod, function: F, generator: G)
    where
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + FnMut(&'lua Lua, A) -> mlua::Result<R>,
        G: Fn(&mut FunctionBuilder<A, R>);

    ///Adds documentation to the next method/function that gets added
    fn document(&mut self, doc: &str) -> &mut Self;
}

/// Typed variant of [`UserDataMethods`]
pub trait TypedDataFields<'lua, T> {
    ///Adds documentation to the next field that gets added
    fn document(&mut self, doc: &str) -> &mut Self;

    /// Typed version of [add_field](mlua::UserDataFields::add_field)
    fn add_field<V>(&mut self, name: impl AsRef<str>, value: V)
    where
        V: IntoLua<'lua> + Clone + 'static + Typed;

    /// Typed version of [add_field_method_get](mlua::UserDataFields::add_field_method_get)
    fn add_field_method_get<S, R, M>(&mut self, name: &S, method: M)
    where
        S: AsRef<str> + ?Sized,
        R: IntoLua<'lua> + Typed,
        M: 'static + MaybeSend + Fn(&'lua Lua, &T) -> mlua::Result<R>;

    /// Typed version of [dd_field_method_set](mlua::UserDataFields::add_field_method_set)
    fn add_field_method_set<S, A, M>(&mut self, name: &S, method: M)
    where
        S: AsRef<str> + ?Sized,
        A: FromLua<'lua> + Typed,
        M: 'static + MaybeSend + FnMut(&'lua Lua, &mut T, A) -> mlua::Result<()>;

    /// Typed version of [add_field_method_get](mlua::UserDataFields::add_field_method_get) and [add_field_method_set](mlua::UserDataFields::add_field_method_set) combined
    fn add_field_method_get_set<S, R, A, GET, SET>(&mut self, name: &S, get: GET, set: SET)
    where
        S: AsRef<str> + ?Sized,
        R: IntoLua<'lua> + Typed,
        A: FromLua<'lua> + Typed,
        GET: 'static + MaybeSend + Fn(&'lua Lua, &T) -> mlua::Result<R>,
        SET: 'static + MaybeSend + Fn(&'lua Lua, &mut T, A) -> mlua::Result<()>;

    /// Typed version of [add_field_function_get](mlua::UserDataFields::add_field_function_get)
    fn add_field_function_get<S, R, F>(&mut self, name: &S, function: F)
    where
        S: AsRef<str> + ?Sized,
        R: IntoLua<'lua> + Typed,
        F: 'static + MaybeSend + Fn(&'lua Lua, AnyUserData<'lua>) -> mlua::Result<R>;

    /// Typed version of [add_field_function_set](mlua::UserDataFields::add_field_function_set)
    fn add_field_function_set<S, A, F>(&mut self, name: &S, function: F)
    where
        S: AsRef<str> + ?Sized,
        A: FromLua<'lua> + Typed,
        F: 'static + MaybeSend + FnMut(&'lua Lua, AnyUserData<'lua>, A) -> mlua::Result<()>;

    /// Typed version of [add_field_function_get](mlua::UserDataFields::add_field_function_get) and [add_field_function_set](mlua::UserDataFields::add_field_function_set) combined
    fn add_field_function_get_set<S, R, A, GET, SET>(&mut self, name: &S, get: GET, set: SET)
    where
        S: AsRef<str> + ?Sized,
        R: IntoLua<'lua> + Typed,
        A: FromLua<'lua> + Typed,
        GET: 'static + MaybeSend + Fn(&'lua Lua, AnyUserData<'lua>) -> mlua::Result<R>,
        SET: 'static + MaybeSend + Fn(&'lua Lua, AnyUserData<'lua>, A) -> mlua::Result<()>;

    /// Typed version of [add_meta_field](mlua::UserDataFields::add_meta_field)
    fn add_meta_field<R, F>(&mut self, meta: MetaMethod, f: F)
    where
        F: 'static + MaybeSend + Fn(&'lua Lua) -> mlua::Result<R>,
        R: IntoLua<'lua> + Typed;
}
