use mlua::{AnyUserData, FromLua, FromLuaMulti, IntoLua, IntoLuaMulti, Lua};
#[cfg(feature = "async")]
use mlua::{UserDataRef, UserDataRefMut};

use crate::{MaybeSend, typed::IntoDocComment};

use super::{Type, Typed, TypedMultiValue};

mod standard;
mod wrapped;

pub use standard::{TypedClass, TypedClassBuilder};
pub use wrapped::WrappedBuilder;

/// Typed variant of [`mlua::UserData`]
pub trait TypedUserData: Sized {
    /// Add documentation to the type itself
    #[allow(unused_variables)]
    fn add_documentation<F: TypedDataDocumentation<Self>>(docs: &mut F) {}

    ///same as [`mlua::UserData::add_methods`].
    ///Refer to its documentation on how to use it.
    ///
    ///only difference is that it takes a [TypedDataMethods],
    ///which is the typed version of [`mlua::UserDataMethods`]
    #[allow(unused_variables)]
    fn add_methods<T: TypedDataMethods<Self>>(methods: &mut T) {}

    /// same as [`mlua::UserData::add_fields`].
    /// Refer to its documentation on how to use it.
    ///
    /// only difference is that it takes a [TypedDataFields],
    /// which is the typed version of [`mlua::UserDataFields`]
    #[allow(unused_variables)]
    fn add_fields<F: TypedDataFields<Self>>(fields: &mut F) {}
}

/// Used inside of [`TypedUserData`] to add doc comments to the userdata type itself
pub trait TypedDataDocumentation<T: TypedUserData> {
    fn add(&mut self, doc: &str) -> &mut Self;
}

/// Typed variant of [`mlua::UserDataMethods`]
pub trait TypedDataMethods<T> {
    /// Exposes a method to lua
    fn add_method<S, A, R, M>(&mut self, name: S, method: M)
    where
        S: Into<String>,
        A: FromLuaMulti + TypedMultiValue,
        R: IntoLuaMulti + TypedMultiValue,
        M: 'static + MaybeSend + Fn(&Lua, &T, A) -> mlua::Result<R>;

    /// Exposes a method to lua that has a mutable reference to Self
    fn add_method_mut<S, A, R, M>(&mut self, name: S, method: M)
    where
        S: Into<String>,
        A: FromLuaMulti + TypedMultiValue,
        R: IntoLuaMulti + TypedMultiValue,
        M: 'static + MaybeSend + FnMut(&Lua, &mut T, A) -> mlua::Result<R>;

    #[cfg(feature = "async")]
    ///exposes an async method to lua
    fn add_async_method<S: Into<String>, A, R, M, MR>(&mut self, name: S, method: M)
    where
        T: 'static,
        M: Fn(Lua, UserDataRef<T>, A) -> MR + MaybeSend + 'static,
        A: FromLuaMulti + TypedMultiValue,
        MR: std::future::Future<Output = mlua::Result<R>> + MaybeSend + 'static,
        R: IntoLuaMulti + TypedMultiValue;

    #[cfg(feature = "async")]
    ///exposes an async method to lua
    fn add_async_method_mut<S: Into<String>, A, R, M, MR>(&mut self, name: S, method: M)
    where
        T: 'static,
        M: Fn(Lua, UserDataRefMut<T>, A) -> MR + MaybeSend + 'static,
        A: FromLuaMulti + TypedMultiValue,
        MR: std::future::Future<Output = mlua::Result<R>> + MaybeSend + 'static,
        R: IntoLuaMulti + TypedMultiValue;

    ///Exposes a function to lua (its a method that does not take Self)
    fn add_function<S, A, R, F>(&mut self, name: S, function: F)
    where
        S: Into<String>,
        A: FromLuaMulti + TypedMultiValue,
        R: IntoLuaMulti + TypedMultiValue,
        F: 'static + MaybeSend + Fn(&Lua, A) -> mlua::Result<R>;

    ///Exposes a mutable function to lua
    fn add_function_mut<S, A, R, F>(&mut self, name: S, function: F)
    where
        S: Into<String>,
        A: FromLuaMulti + TypedMultiValue,
        R: IntoLuaMulti + TypedMultiValue,
        F: 'static + MaybeSend + FnMut(&Lua, A) -> mlua::Result<R>;

    #[cfg(feature = "async")]
    ///exposes an async function to lua
    fn add_async_function<S, A, R, F, FR>(&mut self, name: S, function: F)
    where
        S: Into<String>,
        A: FromLuaMulti + TypedMultiValue,
        R: IntoLuaMulti + TypedMultiValue,
        F: 'static + MaybeSend + Fn(Lua, A) -> FR,
        FR: 'static + MaybeSend + std::future::Future<Output = mlua::Result<R>>;

    ///Exposes a meta method to lua [http://lua-users.org/wiki/MetatableEvents](http://lua-users.org/wiki/MetatableEvents)
    fn add_meta_method<A, R, M>(&mut self, meta: impl Into<String>, method: M)
    where
        A: FromLuaMulti + TypedMultiValue,
        R: IntoLuaMulti + TypedMultiValue,
        M: 'static + MaybeSend + Fn(&Lua, &T, A) -> mlua::Result<R>;

    ///Exposes a meta and mutable method to lua [http://lua-users.org/wiki/MetatableEvents](http://lua-users.org/wiki/MetatableEvents)
    fn add_meta_method_mut<A, R, M>(&mut self, meta: impl Into<String>, method: M)
    where
        A: FromLuaMulti + TypedMultiValue,
        R: IntoLuaMulti + TypedMultiValue,
        M: 'static + MaybeSend + FnMut(&Lua, &mut T, A) -> mlua::Result<R>;

    ///Exposes a meta function to lua [http://lua-users.org/wiki/MetatableEvents](http://lua-users.org/wiki/MetatableEvents)
    fn add_meta_function<A, R, F>(&mut self, meta: impl Into<String>, function: F)
    where
        A: FromLuaMulti + TypedMultiValue,
        R: IntoLuaMulti + TypedMultiValue,
        F: 'static + MaybeSend + Fn(&Lua, A) -> mlua::Result<R>;

    ///Exposes a meta and mutable function to lua [http://lua-users.org/wiki/MetatableEvents](http://lua-users.org/wiki/MetatableEvents)
    fn add_meta_function_mut<A, R, F>(&mut self, meta: impl Into<String>, function: F)
    where
        A: FromLuaMulti + TypedMultiValue,
        R: IntoLuaMulti + TypedMultiValue,
        F: 'static + MaybeSend + FnMut(&Lua, A) -> mlua::Result<R>;

    /// Adds documentation to the next method/function that gets added
    fn document(&mut self, doc: impl IntoDocComment) -> &mut Self;

    /// Adds a param name and doc comment to the next method/function that gets added.
    ///
    /// These will be applied to the params in the order they were defined.
    fn param(&mut self, name: impl std::fmt::Display, doc: impl IntoDocComment) -> &mut Self;

    /// Adds a param name and doc comment to the next method/function that gets added.
    /// Will also add an override type to the param.
    ///
    /// These will be applied to the params in the order they were defined.
    fn param_as(
        &mut self,
        ty: impl Into<Type>,
        name: impl std::fmt::Display,
        doc: impl IntoDocComment,
    ) -> &mut Self;

    /// Adds a return doc comment to the next method/function that gets added.
    ///
    /// These will be applied to the returns in the order they were defined.
    fn ret(&mut self, doc: impl IntoDocComment) -> &mut Self;

    /// Adds a return doc comment to the next method/function that gets added.
    /// Will also add an override type to the return.
    ///
    /// These will be applied to the returns in the order they were defined.
    fn ret_as(&mut self, ty: impl Into<Type>, doc: impl IntoDocComment) -> &mut Self;

    /// Adds an index field with a type and doc comment to the class definition
    fn index<I: Typed>(&mut self, idx: isize, doc: impl IntoDocComment) -> &mut Self;

    /// Adds an index field with a type and doc comment to the class definition
    fn index_as(&mut self, idx: isize, ty: impl Into<Type>, doc: impl IntoDocComment) -> &mut Self;
}

/// Typed variant of [`mlua::UserDataFields`]
pub trait TypedDataFields<T> {
    ///Adds documentation to the next field that gets added
    fn document(&mut self, doc: impl IntoDocComment) -> &mut Self;

    /// Adds a type to the queued overrides.
    ///
    /// It will be used on the next field and will override the type that is automatically used.
    fn coerce(&mut self, ty: impl Into<Type>) -> &mut Self;

    /// Typed version of [add_field](mlua::UserDataFields::add_field)
    fn add_field<V>(&mut self, name: impl Into<String>, value: V)
    where
        V: IntoLua + Clone + 'static + Typed;

    /// Typed version of [add_field_method_get](mlua::UserDataFields::add_field_method_get)
    fn add_field_method_get<S, R, M>(&mut self, name: S, method: M)
    where
        S: Into<String>,
        R: IntoLua + Typed,
        M: 'static + MaybeSend + Fn(&Lua, &T) -> mlua::Result<R>;

    /// Typed version of [dd_field_method_set](mlua::UserDataFields::add_field_method_set)
    fn add_field_method_set<S, A, M>(&mut self, name: S, method: M)
    where
        S: Into<String>,
        A: FromLua + Typed,
        M: 'static + MaybeSend + FnMut(&Lua, &mut T, A) -> mlua::Result<()>;

    /// Typed version of [add_field_method_get](mlua::UserDataFields::add_field_method_get) and [add_field_method_set](mlua::UserDataFields::add_field_method_set) combined
    fn add_field_method_get_set<S, R, A, GET, SET>(&mut self, name: S, get: GET, set: SET)
    where
        S: Into<String>,
        R: IntoLua + Typed,
        A: FromLua + Typed,
        GET: 'static + MaybeSend + Fn(&Lua, &T) -> mlua::Result<R>,
        SET: 'static + MaybeSend + Fn(&Lua, &mut T, A) -> mlua::Result<()>;

    /// Typed version of [add_field_function_get](mlua::UserDataFields::add_field_function_get)
    fn add_field_function_get<S, R, F>(&mut self, name: S, function: F)
    where
        S: Into<String>,
        R: IntoLua + Typed,
        F: 'static + MaybeSend + Fn(&Lua, AnyUserData) -> mlua::Result<R>;

    /// Typed version of [add_field_function_set](mlua::UserDataFields::add_field_function_set)
    fn add_field_function_set<S, A, F>(&mut self, name: S, function: F)
    where
        S: Into<String>,
        A: FromLua + Typed,
        F: 'static + MaybeSend + FnMut(&Lua, AnyUserData, A) -> mlua::Result<()>;

    /// Typed version of [add_field_function_get](mlua::UserDataFields::add_field_function_get) and [add_field_function_set](mlua::UserDataFields::add_field_function_set) combined
    fn add_field_function_get_set<S, R, A, GET, SET>(&mut self, name: S, get: GET, set: SET)
    where
        S: Into<String>,
        R: IntoLua + Typed,
        A: FromLua + Typed,
        GET: 'static + MaybeSend + Fn(&Lua, AnyUserData) -> mlua::Result<R>,
        SET: 'static + MaybeSend + Fn(&Lua, AnyUserData, A) -> mlua::Result<()>;

    /// Typed version of [add_meta_field](mlua::UserDataFields::add_meta_field)
    fn add_meta_field<V>(&mut self, meta: impl Into<String>, value: V)
    where
        V: IntoLua + Typed + 'static;

    /// Typed version of [add_meta_field](mlua::UserDataFields::add_meta_field_with)
    fn add_meta_field_with<R, F>(&mut self, meta: impl Into<String>, f: F)
    where
        F: 'static + MaybeSend + Fn(&Lua) -> mlua::Result<R>,
        R: IntoLua + Typed + 'static;
}
