use mlua::{AnyUserData, FromLua, FromLuaMulti, IntoLua, IntoLuaMulti, Lua, MetaMethod, UserData, UserDataFields, UserDataMethods};

use crate::{typed::generator::FunctionBuilder, MaybeSend};

use super::{Typed, TypedDataFields, TypedDataMethods, TypedMultiValue};

/// Wrapper around a [`UserDataFields`] and [`UserDataMethods`]
/// to allow [`TypedUserData`] implementations to be used for [`UserData`]
/// implementations
pub struct WrappedBuilder<'ctx, U>(&'ctx mut U);
impl<'ctx, U> WrappedBuilder<'ctx, U> {
    pub fn new(u: &'ctx mut U) -> Self {
        WrappedBuilder(u)
    }
}

impl<'lua, 'ctx, T: UserData, U: UserDataFields<'lua, T>> TypedDataFields<'lua, T>
    for WrappedBuilder<'ctx, U>
{
    fn document(&mut self, _doc: &str) -> &mut Self {
        self
    }

    fn add_field<V>(&mut self, name: impl AsRef<str>, value: V)
    where
        V: IntoLua<'lua> + Clone + 'static + Typed,
    {
        self.0.add_field(name, value)
    }

    fn add_field_function_set<S, A, F>(&mut self, name: &S, function: F)
    where
        S: AsRef<str> + ?Sized,
        A: FromLua<'lua> + Typed,
        F: 'static + MaybeSend + FnMut(&'lua Lua, AnyUserData<'lua>, A) -> mlua::Result<()>,
    {
        self.0.add_field_function_set(name, function)
    }

    fn add_field_function_get<S, R, F>(&mut self, name: &S, function: F)
    where
        S: AsRef<str> + ?Sized,
        R: IntoLua<'lua> + Typed,
        F: 'static + MaybeSend + Fn(&'lua Lua, AnyUserData<'lua>) -> mlua::Result<R>,
    {
        self.0.add_field_function_get(name, function)
    }

    fn add_field_function_get_set<S, R, A, GET, SET>(&mut self, name: &S, get: GET, set: SET)
    where
        S: AsRef<str> + ?Sized,
        R: IntoLua<'lua> + Typed,
        A: FromLua<'lua> + Typed,
        GET: 'static + MaybeSend + Fn(&'lua Lua, AnyUserData<'lua>) -> mlua::Result<R>,
        SET: 'static + MaybeSend + Fn(&'lua Lua, AnyUserData<'lua>, A) -> mlua::Result<()>,
    {
        self.0.add_field_function_get(name, get);
        self.0.add_field_function_set(name, set);
    }

    fn add_field_method_set<S, A, M>(&mut self, name: &S, method: M)
    where
        S: AsRef<str> + ?Sized,
        A: FromLua<'lua> + Typed,
        M: 'static + MaybeSend + FnMut(&'lua Lua, &mut T, A) -> mlua::Result<()>,
    {
        self.0.add_field_method_set(name, method)
    }

    fn add_field_method_get<S, R, M>(&mut self, name: &S, method: M)
    where
        S: AsRef<str> + ?Sized,
        R: IntoLua<'lua> + Typed,
        M: 'static + MaybeSend + Fn(&'lua Lua, &T) -> mlua::Result<R>,
    {
        self.0.add_field_method_get(name, method)
    }

    fn add_field_method_get_set<S, R, A, GET, SET>(&mut self, name: &S, get: GET, set: SET)
    where
        S: AsRef<str> + ?Sized,
        R: IntoLua<'lua> + Typed,
        A: FromLua<'lua> + Typed,
        GET: 'static + MaybeSend + Fn(&'lua Lua, &T) -> mlua::Result<R>,
        SET: 'static + MaybeSend + Fn(&'lua Lua, &mut T, A) -> mlua::Result<()>,
    {
        self.0.add_field_method_get(name, get);
        self.0.add_field_method_set(name, set);
    }

    fn add_meta_field<R, F>(&mut self, meta: MetaMethod, f: F)
    where
        F: 'static + MaybeSend + Fn(&'lua Lua) -> mlua::Result<R>,
        R: IntoLua<'lua>,
    {
        self.0.add_meta_field_with(meta, f)
    }
}

impl<'lua, 'ctx, T: UserData, U: UserDataMethods<'lua, T>> TypedDataMethods<'lua, T>
    for WrappedBuilder<'ctx, U>
{
    fn document(&mut self, _documentation: &str) -> &mut Self {
        self
    }

    fn add_method<S, A, R, M>(&mut self, name: &S, method: M)
    where
        S: ?Sized + AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        M: 'static + MaybeSend + Fn(&'lua Lua, &T, A) -> mlua::Result<R>,
    {
        self.0.add_method(name, method)
    }

    fn add_method_with<S, A, R, M, G>(&mut self, name: &S, method: M, _generator: G)
        where
            S: ?Sized + AsRef<str>,
            A: FromLuaMulti<'lua> + TypedMultiValue,
            R: IntoLuaMulti<'lua> + TypedMultiValue,
            M: 'static + MaybeSend + Fn(&'lua Lua, &T, A) -> mlua::Result<R>,
            G: Fn(&mut FunctionBuilder<A, R>) {
        self.0.add_method(name, method)
    }

    fn add_function<S, A, R, F>(&mut self, name: &S, function: F)
    where
        S: ?Sized + AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + Fn(&'lua Lua, A) -> mlua::Result<R>,
    {
        self.0.add_function(name, function)
    }

    fn add_function_with<S, A, R, F, G>(&mut self, name: &S, function: F, _generator: G)
        where
            S: ?Sized + AsRef<str>,
            A: FromLuaMulti<'lua> + TypedMultiValue,
            R: IntoLuaMulti<'lua> + TypedMultiValue,
            F: 'static + MaybeSend + Fn(&'lua Lua, A) -> mlua::Result<R>,
            G: Fn(&mut FunctionBuilder<A, R>) {
        self.0.add_function(name, function)
    }

    fn add_method_mut<S, A, R, M>(&mut self, name: &S, method: M)
    where
        S: ?Sized + AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        M: 'static + MaybeSend + FnMut(&'lua Lua, &mut T, A) -> mlua::Result<R>,
    {
        self.0.add_method_mut(name, method)
    }

    fn add_method_mut_with<S, A, R, M, G>(&mut self, name: &S, method: M, _generator: G)
        where
            S: ?Sized + AsRef<str>,
            A: FromLuaMulti<'lua> + TypedMultiValue,
            R: IntoLuaMulti<'lua> + TypedMultiValue,
            M: 'static + MaybeSend + FnMut(&'lua Lua, &mut T, A) -> mlua::Result<R>,
            G: Fn(&mut FunctionBuilder<A, R>) {
        self.0.add_method_mut(name, method)
    }

    fn add_meta_method<A, R, M>(&mut self, meta: MetaMethod, method: M)
    where
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        M: 'static + MaybeSend + Fn(&'lua Lua, &T, A) -> mlua::Result<R>,
    {
        self.0.add_meta_method(meta, method)
    }

    fn add_meta_method_with<A, R, M, G>(&mut self, meta: MetaMethod, method: M, _generator: G)
        where
            A: FromLuaMulti<'lua> + TypedMultiValue,
            R: IntoLuaMulti<'lua> + TypedMultiValue,
            M: 'static + MaybeSend + Fn(&'lua Lua, &T, A) -> mlua::Result<R>,
            G: Fn(&mut FunctionBuilder<A, R>) {
        self.0.add_meta_method(meta, method)
    }

    #[cfg(feature = "async")]
    fn add_async_method<'s, S: ?Sized + AsRef<str>, A, R, M, MR>(&mut self, name: &S, method: M)
    where
        'lua: 's,
        T: 'static,
        M: Fn(&'lua Lua, &'s T, A) -> MR + MaybeSend + 'static,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        MR: std::future::Future<Output = mlua::Result<R>> + 's,
        R: IntoLuaMulti<'lua>,
    {
        self.0.add_async_method(name, method)
    }

    #[cfg(feature = "async")]
    fn add_async_method_with<'s, S: ?Sized + AsRef<str>, A, R, M, MR, G>(&mut self, name: &S, method: M, _generator: G)
        where
            'lua: 's,
            T: 'static,
            M: Fn(&'lua Lua, &'s T, A) -> MR + MaybeSend + 'static,
            A: FromLuaMulti<'lua> + TypedMultiValue,
            MR: std::future::Future<Output = mlua::Result<R>> + 's,
            R: IntoLuaMulti<'lua> + TypedMultiValue,
            G: Fn(&mut FunctionBuilder<A, R>) {
        self.0.add_async_method(name, method)
    }

    #[cfg(feature = "async")]
    fn add_async_method_mut<'s, S: ?Sized + AsRef<str>, A, R, M, MR>(&mut self, name: &S, method: M)
    where
        'lua: 's,
        T: 'static,
        M: Fn(&'lua Lua, &'s mut T, A) -> MR + MaybeSend + 'static,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        MR: std::future::Future<Output = mlua::Result<R>> + 's,
        R: IntoLuaMulti<'lua>,
    {
        self.0.add_async_method_mut(name, method)
    }

    #[cfg(feature = "async")]
    fn add_async_method_mut_with<'s, S: ?Sized + AsRef<str>, A, R, M, MR, G>(&mut self, name: &S, method: M, _generator: G)
        where
            'lua: 's,
            T: 'static,
            M: Fn(&'lua Lua, &'s mut T, A) -> MR + MaybeSend + 'static,
            A: FromLuaMulti<'lua> + TypedMultiValue,
            MR: std::future::Future<Output = mlua::Result<R>> + 's,
            R: IntoLuaMulti<'lua> + TypedMultiValue,
            G: Fn(&mut FunctionBuilder<A, R>) {
        self.0.add_async_method_mut(name, method)
    }

    fn add_function_mut<S, A, R, F>(&mut self, name: &S, function: F)
    where
        S: ?Sized + AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + FnMut(&'lua Lua, A) -> mlua::Result<R>,
    {
        self.0.add_function_mut(name, function)
    }

    fn add_function_mut_with<S, A, R, F, G>(&mut self, name: &S, function: F, _generator: G)
        where
            S: ?Sized + AsRef<str>,
            A: FromLuaMulti<'lua> + TypedMultiValue,
            R: IntoLuaMulti<'lua> + TypedMultiValue,
            F: 'static + MaybeSend + FnMut(&'lua Lua, A) -> mlua::Result<R>,
            G: Fn(&mut FunctionBuilder<A, R>) {
        self.0.add_function_mut(name, function)
    }

    fn add_meta_function<A, R, F>(&mut self, meta: MetaMethod, function: F)
    where
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + Fn(&'lua Lua, A) -> mlua::Result<R>,
    {
        self.0.add_meta_function(meta, function)
    }

    fn add_meta_function_with<A, R, F, G>(&mut self, meta: MetaMethod, function: F, _generator: G)
        where
            A: FromLuaMulti<'lua> + TypedMultiValue,
            R: IntoLuaMulti<'lua> + TypedMultiValue,
            F: 'static + MaybeSend + Fn(&'lua Lua, A) -> mlua::Result<R>,
            G: Fn(&mut FunctionBuilder<A, R>) {
        self.0.add_meta_function(meta, function)
    }

    #[cfg(feature = "async")]
    fn add_async_function<S: ?Sized, A, R, F, FR>(&mut self, name: &S, function: F)
    where
        S: AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + Fn(&'lua Lua, A) -> FR,
        FR: 'lua + std::future::Future<Output = mlua::Result<R>>,
    {
        self.0.add_async_function(name, function)
    }

    #[cfg(feature = "async")]
    fn add_async_function_with<S: ?Sized, A, R, F, FR, G>(&mut self, name: &S, function: F, _generator: G)
        where
            S: AsRef<str>,
            A: FromLuaMulti<'lua> + TypedMultiValue,
            R: IntoLuaMulti<'lua> + TypedMultiValue,
            F: 'static + MaybeSend + Fn(&'lua Lua, A) -> FR,
            FR: 'lua + std::future::Future<Output = mlua::Result<R>>,
            G: Fn(&mut FunctionBuilder<A, R>) {
        self.0.add_async_function(name, function)
    }

    fn add_meta_method_mut<A, R, M>(&mut self, meta: MetaMethod, method: M)
    where
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        M: 'static + MaybeSend + FnMut(&'lua Lua, &mut T, A) -> mlua::Result<R>,
    {
        self.0.add_meta_method_mut(meta, method)
    }

    fn add_meta_method_mut_with<A, R, M, G>(&mut self, meta: MetaMethod, method: M, _generator: G)
        where
            A: FromLuaMulti<'lua> + TypedMultiValue,
            R: IntoLuaMulti<'lua> + TypedMultiValue,
            M: 'static + MaybeSend + FnMut(&'lua Lua, &mut T, A) -> mlua::Result<R>,
            G: Fn(&mut FunctionBuilder<A, R>) {
        self.0.add_meta_method_mut(meta, method)
    }

    fn add_meta_function_mut<A, R, F>(&mut self, meta: MetaMethod, function: F)
    where
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + FnMut(&'lua Lua, A) -> mlua::Result<R>,
    {
        self.0.add_meta_function_mut(meta, function)
    }

    fn add_meta_function_mut_with<A, R, F, G>(&mut self, meta: MetaMethod, function: F, _generator: G)
        where
            A: FromLuaMulti<'lua> + TypedMultiValue,
            R: IntoLuaMulti<'lua> + TypedMultiValue,
            F: 'static + MaybeSend + FnMut(&'lua Lua, A) -> mlua::Result<R>,
            G: Fn(&mut FunctionBuilder<A, R>) {
        self.0.add_meta_function_mut(meta, function)
    }
}
