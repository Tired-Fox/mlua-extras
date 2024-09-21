use std::{borrow::Cow, marker::PhantomData};

use mlua::{FromLua, FromLuaMulti, Function, IntoLua, IntoLuaMulti, Lua, Value};

use super::{MaybeSend, Type, Typed, TypedMultiValue};

/// A function parameter type representation
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct Param {
    ///If the parameter has a name (will default to Param{number} if None)
    pub name: Option<Cow<'static, str>>,
    ///The type of the parameter
    pub ty: Type,
}

impl<I: Into<Cow<'static, str>>> From<(I, Type)> for Param {
    fn from((name, ty): (I, Type)) -> Self {
        Param {
            name: Some(name.into()),
            ty,
        }
    }
}

impl From<Type> for Param {
    fn from(value: Type) -> Self {
        Param {
            name: None,
            ty: value,
        }
    }
}

/// Used to purely get function type information without converting it to anything
/// else.
pub trait IntoTypedFunction<'lua, Params: TypedMultiValue, Response: TypedMultiValue> {
    fn into_typed_function(
        self,
        lua: &'lua Lua,
    ) -> mlua::Result<TypedFunction<'lua, Params, Response>>;
}

impl<'lua, F, Params, Response> IntoTypedFunction<'lua, Params, Response> for F
where
    Params: TypedMultiValue + FromLuaMulti<'lua>,
    Response: TypedMultiValue + IntoLuaMulti<'lua>,
    F: Fn(&'lua Lua, Params) -> mlua::Result<Response> + MaybeSend + 'static,
{
    fn into_typed_function(
        self,
        lua: &'lua Lua,
    ) -> mlua::Result<TypedFunction<'lua, Params, Response>> {
        Ok(TypedFunction {
            inner: lua.create_function(self)?,
            _p: PhantomData,
            _r: PhantomData,
        })
    }
}

impl<'lua, Params, Response> IntoTypedFunction<'lua, Params, Response> for Function<'lua>
where
    Params: TypedMultiValue + FromLuaMulti<'lua>,
    Response: TypedMultiValue + IntoLuaMulti<'lua>,
{
    fn into_typed_function(
        self,
        _lua: &'lua Lua,
    ) -> mlua::Result<TypedFunction<'lua, Params, Response>> {
        Ok(TypedFunction {
            inner: self,
            _p: PhantomData,
            _r: PhantomData,
        })
    }
}

impl<'lua, Params, Response> IntoTypedFunction<'lua, Params, Response>
    for &TypedFunction<'lua, Params, Response>
where
    Params: TypedMultiValue + FromLuaMulti<'lua>,
    Response: TypedMultiValue + IntoLuaMulti<'lua>,
{
    fn into_typed_function(
        self,
        _lua: &'lua Lua,
    ) -> mlua::Result<TypedFunction<'lua, Params, Response>> {
        Ok(TypedFunction {
            inner: self.inner.clone(),
            _p: PhantomData,
            _r: PhantomData,
        })
    }
}

impl<'lua, Params, Response> IntoTypedFunction<'lua, Params, Response> for ()
where
    Params: TypedMultiValue + FromLuaMulti<'lua>,
    Response: TypedMultiValue + IntoLuaMulti<'lua>,
{
    fn into_typed_function(
        self,
        lua: &'lua Lua,
    ) -> mlua::Result<TypedFunction<'lua, Params, Response>> {
        Ok(TypedFunction {
            inner: lua.create_function(|_, _: Params| Ok(()))?,
            _p: PhantomData,
            _r: PhantomData,
        })
    }
}

/// Helper to bake the type information for a lua [`Function`][mlua::Function]. This makes repeated
/// calls to the [`Function`][mlua::Function]'s [`call`][mlua::Function::call] all the same with
/// enforced arguments and return types.
pub struct TypedFunction<'lua, Params, Response>
where
    Params: TypedMultiValue,
    Response: TypedMultiValue,
{
    inner: Function<'lua>,
    _p: PhantomData<Params>,
    _r: PhantomData<Response>,
}

impl<'lua, Params, Response> TypedFunction<'lua, Params, Response>
where
    Params: TypedMultiValue + IntoLuaMulti<'lua>,
    Response: TypedMultiValue + FromLuaMulti<'lua>,
{
    /// Same as [rlua::Function::call](rlua::Function#method.call) but with the param and return
    /// types already specified
    pub fn call(&self, params: Params) -> mlua::Result<Response> {
        self.inner.call::<Params, Response>(params)
    }

    /// Same as [rlua::Function::call](rlua::Function#method.call) but with the param and return
    /// types already specified
    ///
    /// # Safety
    ///
    /// Panics if any lua errors occur
    pub unsafe fn call_unsafe(&self, params: Params) -> Response {
        self.inner.call::<Params, Response>(params).unwrap()
    }

    /// Create a typed function from a rust function.
    ///
    /// This will call [`Lua::create_function`][mlua::Lua::create_function] under the hood
    pub fn from_rust<F>(&self, lua: &'lua Lua, func: F) -> mlua::Result<Self>
    where
        Params: TypedMultiValue + FromLuaMulti<'lua>,
        Response: TypedMultiValue + IntoLuaMulti<'lua>,
        F: Fn(&'lua Lua, Params) -> mlua::Result<Response> + MaybeSend + 'static,
    {
        Ok(Self {
            inner: lua.create_function(func)?,
            _p: PhantomData,
            _r: PhantomData,
        })
    }
}

impl<'lua, Params, Response> FromLua<'lua> for TypedFunction<'lua, Params, Response>
where
    Params: TypedMultiValue,
    Response: TypedMultiValue,
{
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> mlua::prelude::LuaResult<Self> {
        Ok(Self {
            inner: FromLua::from_lua(value, lua)?,
            _p: PhantomData,
            _r: PhantomData,
        })
    }
}

impl<'lua, Params, Response> IntoLua<'lua> for TypedFunction<'lua, Params, Response>
where
    Params: TypedMultiValue,
    Response: TypedMultiValue,
{
    fn into_lua(self, _lua: &'lua Lua) -> mlua::prelude::LuaResult<Value<'lua>> {
        Ok(Value::Function(self.inner))
    }
}

impl<'lua, Params, Response> Typed for TypedFunction<'lua, Params, Response>
where
    Params: TypedMultiValue,
    Response: TypedMultiValue,
{
    fn ty() -> Type {
        Type::Function {
            params: Params::get_types_as_params(),
            returns: Response::get_types(),
        }
    }
}
