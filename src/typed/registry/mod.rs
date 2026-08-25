use std::{borrow::Cow, collections::BTreeMap, marker::PhantomData};

use mlua::{AnyUserData, FromLua, FromLuaMulti, IntoLua, IntoLuaMulti, Lua};
#[cfg(feature = "async")]
use mlua::{UserDataRef, UserDataRefMut};

use crate::{
    MaybeSend,
    ser::to_lua_repr,
    typed::{Field, Func, Index, IntoDocComment, StaticField, Type, Typed, TypedMultiValue},
};

pub mod wrapper;

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawTypedUserDataRegistry {
    pub type_name: Cow<'static, str>,
    pub type_doc: Option<Cow<'static, str>>,

    pub derives: Vec<Cow<'static, str>>,

    pub fields: BTreeMap<Index, Field>,
    pub static_fields: BTreeMap<Index, StaticField>,
    pub meta_fields: BTreeMap<Index, Field>,
    pub static_meta_fields: BTreeMap<Index, StaticField>,

    pub methods: BTreeMap<Index, Func>,
    pub meta_methods: BTreeMap<Index, Func>,

    pub functions: BTreeMap<Index, Func>,
    pub meta_functions: BTreeMap<Index, Func>,
}
impl RawTypedUserDataRegistry {
    pub fn is_meta_empty(&self) -> bool {
        self.meta_fields.is_empty()
            && self.static_meta_fields.is_empty()
            && self.meta_functions.is_empty()
            && self.meta_methods.is_empty()
    }
}

/// Type information for a lua `UserData`. This happens to be a [`TypedUserData`]
#[derive(Debug, Clone)]
pub struct TypedUserDataRegistry<T = ()> {
    lua: Lua,
    pub(crate) raw: RawTypedUserDataRegistry,

    queued_doc: Option<Cow<'static, str>>,
    queued_ty: Option<Type>,
    queued_params: Vec<(Option<Type>, String, Option<Cow<'static, str>>)>,
    queued_returns: Vec<(Option<Type>, Option<Cow<'static, str>>)>,

    _phantom: PhantomData<T>,
}
impl<T> Default for TypedUserDataRegistry<T> {
    fn default() -> Self {
        Self {
            lua: Default::default(),
            raw: Default::default(),
            queued_doc: Default::default(),
            queued_params: Default::default(),
            queued_returns: Default::default(),
            queued_ty: Default::default(),
            _phantom: PhantomData,
        }
    }
}

// TODO: Change Type::Class to be TypedUserDataRegistry
// impl From<TypedUserDataRegistry> for Type {
//     fn from(value: TypedUserDataRegistry) -> Self {
//         Type::Class(Box::new(value.raw))
//     }
// }

impl TypedUserDataRegistry<()> {
    pub fn new<U: TypedUserData>() -> TypedUserDataRegistry<U> {
        let mut registry = TypedUserDataRegistry::<U>::default();
        U::register(&mut registry);
        registry
    }

    pub fn any() -> TypedUserDataRegistry<()> {
        Self {
            lua: Default::default(),
            raw: Default::default(),
            queued_doc: Default::default(),
            queued_params: Default::default(),
            queued_returns: Default::default(),
            queued_ty: Default::default(),
            _phantom: PhantomData,
        }
    }
}

impl<T> TypedUserDataRegistry<T> {
    pub fn build(self) -> RawTypedUserDataRegistry {
        self.raw
    }

    /// Creates a new typed field and adds it to the class's type information
    ///
    /// # Example
    ///
    /// ```
    /// use mlua_extras::typed::{TypedUserDataRegistry, Type};
    ///
    /// static NAME: &str = "mlua_extras";
    ///
    /// TypedUserDataRegistry::any()
    ///     .field("data1", Type::string() | Type::nil(), "doc comment goes last")
    ///     .field("data2", Type::array(Type::string()), ()) // Can also use `None` instead of `()`
    ///     .field("message", Type::string(), format!("A message for {NAME}"));
    /// ```
    pub fn field(mut self, key: impl Into<Index>, ty: Type, doc: impl IntoDocComment) -> Self {
        self.raw.fields.insert(key.into(), Field::new(ty, doc));
        self
    }

    pub fn static_field<V>(
        mut self,
        key: impl Into<Index>,
        value: V,
        doc: impl IntoDocComment,
    ) -> Self
    where
        V: Typed + IntoLua,
    {
        let value = match value.into_lua(&self.lua) {
            Ok(value) => to_lua_repr(&value).map_err(mlua::Error::runtime),
            Err(err) => Err(err),
        };

        if let Ok(value) = value {
            self.raw
                .static_fields
                .insert(key.into(), StaticField::new(V::ty(), doc, value));
        }
        self
    }

    pub fn inherit(mut self, parent: &RawTypedUserDataRegistry) -> Self {
        self.raw.static_fields.extend(parent.static_fields.clone());
        self
    }

    /// Creates a new typed function and adds it to the class's type information
    ///
    /// # Example
    ///
    /// ```
    /// use mlua_extras::typed::{TypedUserDataRegistry, Type};
    ///
    /// TypedUserDataRegistry::any()
    ///     .function::<String, ()>("greet", "Greet the given name")
    ///     // Can use `None` instead of `()` for specifying the doc comment
    ///     .function::<String, ()>("hello", ());
    /// ```
    pub fn function<Params, Returns>(
        mut self,
        key: impl Into<Index>,
        doc: impl IntoDocComment,
    ) -> Self
    where
        Params: TypedMultiValue,
        Returns: TypedMultiValue,
    {
        self.raw.functions.insert(
            key.into(),
            Func::new::<Params, Returns>(
                doc,
                self.queued_params.drain(..).collect(),
                self.queued_returns.drain(..).collect(),
            ),
        );
        self
    }

    /// Creates a new typed method and adds it to the class's type information.
    ///
    /// As with methods in lua, the `self` parameter is implicit and has the same type as the
    /// parent class.
    ///
    /// # Example
    ///
    /// ```
    /// use mlua_extras::typed::{TypedUserDataRegistry, Type};
    ///
    /// TypedUserDataRegistry::any()
    ///     .method::<String, ()>("greet", "Greet the given name")
    ///     // Can use `None` instead of `()` for specifying the doc comment
    ///     .method::<String, ()>("hello", ());
    /// ```
    pub fn method<Params, Returns>(
        mut self,
        key: impl Into<Index>,
        doc: impl IntoDocComment,
    ) -> Self
    where
        Params: TypedMultiValue,
        Returns: TypedMultiValue,
    {
        self.raw.methods.insert(
            key.into(),
            Func::new::<Params, Returns>(
                doc,
                self.queued_params.drain(..).collect(),
                self.queued_returns.drain(..).collect(),
            ),
        );
        self
    }

    /// Creates a new typed field and adds it to the class's meta type information
    ///
    /// # Example
    ///
    /// ```
    /// use mlua_extras::typed::{TypedUserDataRegistry, Type};
    ///
    /// static NAME: &str = "mlua_extras";
    ///
    /// TypedUserDataRegistry::any()
    ///     .meta_field("data1", Type::string() | Type::nil(), "doc comment goes last")
    ///     .meta_field("data2", Type::array(Type::string()), ()) // Can also use `None` instead of `()`
    ///     .meta_field("message", Type::string(), format!("A message for {NAME}"));
    /// ```
    pub fn meta_field(mut self, key: impl Into<Index>, ty: Type, doc: impl IntoDocComment) -> Self {
        self.raw.meta_fields.insert(key.into(), Field::new(ty, doc));
        self
    }

    /// Creates a new typed function and adds it to the class's meta type information
    ///
    /// # Example
    ///
    /// ```
    /// use mlua_extras::typed::{TypedUserDataRegistry, Type};
    ///
    /// TypedUserDataRegistry::any()
    ///     .meta_function::<String, ()>("greet", "Greet the given name")
    ///     // Can use `None` instead of `()` for specifying the doc comment
    ///     .meta_function::<String, ()>("hello", ());
    /// ```
    pub fn meta_function<Params, Returns>(
        mut self,
        key: impl Into<Index>,
        doc: impl IntoDocComment,
    ) -> Self
    where
        Params: TypedMultiValue,
        Returns: TypedMultiValue,
    {
        self.raw.meta_functions.insert(
            key.into(),
            Func::new::<Params, Returns>(
                doc,
                self.queued_params.drain(..).collect(),
                self.queued_returns.drain(..).collect(),
            ),
        );
        self
    }

    /// Creates a new typed method and adds it to the class's type information.
    ///
    /// As with methods in lua, the `self` parameter is implicit and has the same type as the
    /// parent class.
    ///
    /// # Example
    ///
    /// ```
    /// use mlua_extras::typed::{TypedUserDataRegistry, Type};
    ///
    /// static NAME: &str = "mlua_extras";
    ///
    /// TypedUserDataRegistry::any()
    ///     .method::<String, ()>("greet", "Greet the given name")
    ///     // Can use `None` instead of `()` for specifying the doc comment
    ///     .method::<String, ()>("hello", ());
    /// ```
    pub fn meta_method<Params, Returns>(
        mut self,
        key: impl Into<Index>,
        doc: impl IntoDocComment,
    ) -> Self
    where
        Params: TypedMultiValue,
        Returns: TypedMultiValue,
    {
        self.raw.meta_methods.insert(
            key.into(),
            Func::new::<Params, Returns>(
                doc,
                self.queued_params.drain(..).collect(),
                self.queued_returns.drain(..).collect(),
            ),
        );
        self
    }

    /// Add a child class that this class derives
    pub fn derive(mut self, parent: impl std::fmt::Display) -> Self {
        self.raw.derives.push(parent.to_string().into());
        self
    }
}

/// Typed variant of [`mlua::UserData`]
pub trait TypedUserData: Sized {
    /// Add documentation to the type itself
    #[allow(unused_variables)]
    fn add_documentation<F: TypedDataDocumentation<Self>>(docs: &mut F) {}

    /// Same as [`mlua::UserData::add_methods`].
    /// Refer to its documentation on how to use it.
    ///
    /// only difference is that it takes a [TypedDataMethods],
    /// which is the typed version of [`mlua::UserDataMethods`]
    #[allow(unused_variables)]
    fn add_methods<T: TypedDataMethods<Self>>(methods: &mut T) {}

    /// Same as [`mlua::UserData::add_fields`].
    /// Refer to its documentation on how to use it.
    ///
    /// only difference is that it takes a [TypedDataFields],
    /// which is the typed version of [`mlua::UserDataFields`]
    #[allow(unused_variables)]
    fn add_fields<F: TypedDataFields<Self>>(fields: &mut F) {}

    /// Same as [`mlua::UserData::register`].
    /// Refer to its documentation on how to use it.
    ///
    /// Only difference is that it takes a [`TypedUserDataRegistry`],
    /// which is the typed version of [`mlua::UserDataRegistry`]
    fn register(registry: &mut TypedUserDataRegistry<Self>) {
        Self::add_documentation(registry);
        Self::add_fields(registry);
        Self::add_methods(registry);
    }
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

impl<T: TypedUserData> TypedDataDocumentation<T> for TypedUserDataRegistry<T> {
    fn add(&mut self, doc: &str) -> &mut Self {
        if let Some(type_doc) = self.raw.type_doc.as_mut() {
            *type_doc = format!("{type_doc}\n{doc}").into()
        } else {
            self.raw.type_doc = Some(doc.to_string().into())
        }
        self
    }
}

impl<T: TypedUserData> TypedDataFields<T> for TypedUserDataRegistry<T> {
    fn document(&mut self, doc: impl IntoDocComment) -> &mut Self {
        self.queued_doc = doc.into_doc_comment();
        self
    }

    fn coerce(&mut self, ty: impl Into<Type>) -> &mut Self {
        self.queued_ty = Some(ty.into());
        self
    }

    fn add_field<V>(&mut self, name: impl Into<String>, value: V)
    where
        V: IntoLua + Clone + 'static + Typed,
    {
        let value = match value.into_lua(&self.lua) {
            Ok(value) => to_lua_repr(&value).map_err(mlua::Error::runtime),
            Err(err) => Err(err),
        };

        if let Ok(value) = value {
            let name: Cow<'static, str> = name.into().into();
            let ty = self.queued_ty.take().unwrap_or(V::as_param());
            let value: Cow<'static, str> = value.into();

            self.raw.static_fields.insert(
                name.into(),
                StaticField::new(ty, self.queued_doc.take(), value),
            );
        }
    }

    fn add_field_function_set<S, A, F>(&mut self, name: S, _: F)
    where
        S: Into<String>,
        A: FromLua + Typed,
        F: 'static + MaybeSend + FnMut(&Lua, AnyUserData, A) -> mlua::Result<()>,
    {
        let name: Cow<'static, str> = name.into().into();
        let ty = self.queued_ty.take().unwrap_or(A::as_param());
        self.raw
            .fields
            .entry(name.into())
            .and_modify({
                let ty = ty.clone();
                |v| {
                    if let Some(doc) = self.queued_doc.take() {
                        v.doc = Some(match v.doc.take() {
                            Some(d) => format!("{d}\n{doc}").into(),
                            None => doc,
                        });
                    }
                    v.ty = v.ty.clone() | ty;
                }
            })
            .or_insert(Field {
                ty,
                doc: self.queued_doc.take().map(|v| v.into()),
            });
    }

    fn add_field_function_get<S, R, F>(&mut self, name: S, _: F)
    where
        S: Into<String>,
        R: IntoLua + Typed,
        F: 'static + MaybeSend + Fn(&Lua, AnyUserData) -> mlua::Result<R>,
    {
        let name: Cow<'static, str> = name.into().into();
        let ty = self.queued_ty.take().unwrap_or(R::as_return());
        self.raw
            .fields
            .entry(name.into())
            .and_modify({
                let ty = ty.clone();
                |v| {
                    if let Some(doc) = self.queued_doc.take() {
                        v.doc = Some(match v.doc.take() {
                            Some(d) => format!("{d}\n{doc}").into(),
                            None => doc,
                        });
                    }
                    v.ty = v.ty.clone() | ty;
                }
            })
            .or_insert(Field {
                ty,
                doc: self.queued_doc.take().map(|v| v.into()),
            });
    }

    fn add_field_function_get_set<S, R, A, GET, SET>(&mut self, name: S, _: GET, _: SET)
    where
        S: Into<String>,
        R: IntoLua + Typed,
        A: FromLua + Typed,
        GET: 'static + MaybeSend + Fn(&Lua, AnyUserData) -> mlua::Result<R>,
        SET: 'static + MaybeSend + Fn(&Lua, AnyUserData, A) -> mlua::Result<()>,
    {
        let name: Cow<'static, str> = name.into().into();
        let ty = self
            .queued_ty
            .take()
            .unwrap_or(A::as_param() | R::as_return());
        self.raw
            .fields
            .entry(name.into())
            .and_modify({
                let ty = ty.clone();
                |v| {
                    if let Some(doc) = self.queued_doc.take() {
                        v.doc = Some(match v.doc.take() {
                            Some(d) => format!("{d}\n{doc}").into(),
                            None => doc,
                        });
                    }
                    v.ty = v.ty.clone() | ty;
                }
            })
            .or_insert(Field {
                ty,
                doc: self.queued_doc.take().map(|v| v.into()),
            });
    }

    fn add_field_method_set<S, A, M>(&mut self, name: S, _: M)
    where
        S: Into<String>,
        A: FromLua + Typed,
        M: 'static + MaybeSend + FnMut(&Lua, &mut T, A) -> mlua::Result<()>,
    {
        let name: Cow<'static, str> = name.into().into();
        let ty = self.queued_ty.take().unwrap_or(A::as_param());
        self.raw
            .fields
            .entry(name.into())
            .and_modify({
                let ty = ty.clone();
                |v| {
                    if let Some(doc) = self.queued_doc.take() {
                        v.doc = Some(match v.doc.take() {
                            Some(d) => format!("{d}\n{doc}").into(),
                            None => doc,
                        });
                    }
                    v.ty = v.ty.clone() | ty;
                }
            })
            .or_insert(Field {
                ty,
                doc: self.queued_doc.take().map(|v| v.into()),
            });
    }

    fn add_field_method_get<S, R, M>(&mut self, name: S, _: M)
    where
        S: Into<String>,
        R: IntoLua + Typed,
        M: 'static + MaybeSend + Fn(&Lua, &T) -> mlua::Result<R>,
    {
        let name: Cow<'static, str> = name.into().into();
        let ty = self.queued_ty.take().unwrap_or(R::as_return());
        self.raw
            .fields
            .entry(name.into())
            .and_modify({
                let ty = ty.clone();
                |v| {
                    if let Some(doc) = self.queued_doc.take() {
                        v.doc = Some(match v.doc.take() {
                            Some(d) => format!("{d}\n{doc}").into(),
                            None => doc,
                        });
                    }
                    v.ty = v.ty.clone() | ty;
                }
            })
            .or_insert(Field {
                ty,
                doc: self.queued_doc.take().map(|v| v.into()),
            });
    }

    fn add_field_method_get_set<S, R, A, GET, SET>(&mut self, name: S, _: GET, _: SET)
    where
        S: Into<String>,
        R: IntoLua + Typed,
        A: FromLua + Typed,
        GET: 'static + MaybeSend + Fn(&Lua, &T) -> mlua::Result<R>,
        SET: 'static + MaybeSend + Fn(&Lua, &mut T, A) -> mlua::Result<()>,
    {
        let name: Cow<'static, str> = name.into().into();
        let ty = self
            .queued_ty
            .take()
            .unwrap_or(A::as_param() | R::as_return());
        self.raw
            .fields
            .entry(name.into())
            .and_modify({
                let ty = ty.clone();
                |v| {
                    if let Some(doc) = self.queued_doc.take() {
                        v.doc = Some(match v.doc.take() {
                            Some(d) => format!("{d}\n{doc}").into(),
                            None => doc,
                        });
                    }
                    v.ty = v.ty.clone() | ty;
                }
            })
            .or_insert(Field {
                ty,
                doc: self.queued_doc.take().map(|v| v.into()),
            });
    }

    fn add_meta_field<V>(&mut self, meta: impl Into<String>, value: V)
    where
        V: IntoLua + Typed + 'static,
    {
        let value = match value.into_lua(&self.lua) {
            Ok(value) => to_lua_repr(&value).map_err(mlua::Error::runtime),
            Err(err) => Err(err),
        };

        if let Ok(value) = value {
            let name: Cow<'static, str> = meta.into().into();
            let ty = self.queued_ty.take().unwrap_or(V::as_param());
            let value: Cow<'static, str> = value.into();

            self.raw.static_meta_fields.insert(
                name.into(),
                StaticField::new(ty, self.queued_doc.take(), value),
            );
        }
    }

    fn add_meta_field_with<R, F>(&mut self, meta: impl Into<String>, _: F)
    where
        F: 'static + MaybeSend + Fn(&Lua) -> mlua::Result<R>,
        R: IntoLua + Typed,
    {
        let name: Cow<'static, str> = meta.into().into();
        let ty = self.queued_ty.take().unwrap_or(R::as_return());
        self.raw
            .meta_fields
            .entry(name.into())
            .and_modify({
                let ty = ty.clone();
                |v| {
                    if let Some(doc) = self.queued_doc.take() {
                        v.doc = Some(match v.doc.take() {
                            Some(d) => format!("{d}\n{doc}").into(),
                            None => doc,
                        });
                    }
                    v.ty = v.ty.clone() | ty;
                }
            })
            .or_insert(Field {
                ty,
                doc: self.queued_doc.take().map(|v| v.into()),
            });
    }
}

impl<T: TypedUserData> TypedDataMethods<T> for TypedUserDataRegistry<T> {
    fn document(&mut self, doc: impl IntoDocComment) -> &mut Self {
        self.queued_doc = doc.into_doc_comment();
        self
    }

    fn param(&mut self, name: impl std::fmt::Display, doc: impl IntoDocComment) -> &mut Self {
        self.queued_params
            .push((None, name.to_string(), doc.into_doc_comment()));
        self
    }

    fn param_as(
        &mut self,
        ty: impl Into<Type>,
        name: impl std::fmt::Display,
        doc: impl IntoDocComment,
    ) -> &mut Self {
        self.queued_params
            .push((Some(ty.into()), name.to_string(), doc.into_doc_comment()));
        self
    }

    fn ret(&mut self, doc: impl IntoDocComment) -> &mut Self {
        if let Some(doc) = doc.into_doc_comment() {
            self.queued_returns.push((None, Some(doc)));
        }
        self
    }

    fn ret_as(&mut self, ty: impl Into<Type>, doc: impl IntoDocComment) -> &mut Self {
        self.queued_returns
            .push((Some(ty.into()), doc.into_doc_comment()));
        self
    }

    fn index<I: Typed>(&mut self, idx: isize, doc: impl IntoDocComment) -> &mut Self {
        self.raw.fields.insert(
            idx.into(),
            Field {
                ty: I::as_param(),
                doc: doc.into_doc_comment(),
            },
        );
        self
    }

    fn index_as(&mut self, idx: isize, ty: impl Into<Type>, doc: impl IntoDocComment) -> &mut Self {
        self.raw.fields.insert(
            idx.into(),
            Field {
                ty: ty.into(),
                doc: doc.into_doc_comment(),
            },
        );
        self
    }

    fn add_method<S, A, R, M>(&mut self, name: S, _: M)
    where
        S: Into<String>,
        A: FromLuaMulti + TypedMultiValue,
        R: IntoLuaMulti + TypedMultiValue,
        M: 'static + MaybeSend + Fn(&Lua, &T, A) -> mlua::Result<R>,
    {
        let name: Cow<'static, str> = name.into().into();
        self.raw.methods.insert(
            name.into(),
            Func::new::<A, R>(
                self.queued_doc.take(),
                self.queued_params.drain(..).collect(),
                self.queued_returns.drain(..).collect(),
            ),
        );
    }

    fn add_function<S, A, R, F>(&mut self, name: S, _: F)
    where
        S: Into<String>,
        A: FromLuaMulti + TypedMultiValue,
        R: IntoLuaMulti + TypedMultiValue,
        F: 'static + MaybeSend + Fn(&Lua, A) -> mlua::Result<R>,
    {
        let name: Cow<'static, str> = name.into().into();
        self.raw.functions.insert(
            name.into(),
            Func::new::<A, R>(
                self.queued_doc.take(),
                self.queued_params.drain(..).collect(),
                self.queued_returns.drain(..).collect(),
            ),
        );
    }

    fn add_method_mut<S, A, R, M>(&mut self, name: S, _: M)
    where
        S: Into<String>,
        A: FromLuaMulti + TypedMultiValue,
        R: IntoLuaMulti + TypedMultiValue,
        M: 'static + MaybeSend + FnMut(&Lua, &mut T, A) -> mlua::Result<R>,
    {
        let name: Cow<'static, str> = name.into().into();
        self.raw.methods.insert(
            name.into(),
            Func::new::<A, R>(
                self.queued_doc.take(),
                self.queued_params.drain(..).collect(),
                self.queued_returns.drain(..).collect(),
            ),
        );
    }

    fn add_meta_method<A, R, M>(&mut self, meta: impl Into<String>, _: M)
    where
        A: FromLuaMulti + TypedMultiValue,
        R: IntoLuaMulti + TypedMultiValue,
        M: 'static + MaybeSend + Fn(&Lua, &T, A) -> mlua::Result<R>,
    {
        let name: Cow<'static, str> = meta.into().into();
        self.raw.meta_methods.insert(
            name.into(),
            Func::new::<A, R>(
                self.queued_doc.take(),
                self.queued_params.drain(..).collect(),
                self.queued_returns.drain(..).collect(),
            ),
        );
    }

    #[cfg(feature = "async")]
    fn add_async_method<S: Into<String>, A, R, M, MR>(&mut self, name: S, _: M)
    where
        T: 'static,
        M: Fn(Lua, mlua::UserDataRef<T>, A) -> MR + MaybeSend + 'static,
        A: FromLuaMulti + TypedMultiValue,
        MR: std::future::Future<Output = mlua::Result<R>> + MaybeSend + 'static,
        R: IntoLuaMulti + TypedMultiValue,
    {
        let name: Cow<'static, str> = name.into().into();
        self.raw.methods.insert(
            name.into(),
            Func::new::<A, R>(
                self.queued_doc.take(),
                self.queued_params.drain(..).collect(),
                self.queued_returns.drain(..).collect(),
            ),
        );
    }

    #[cfg(feature = "async")]
    fn add_async_method_mut<S: Into<String>, A, R, M, MR>(&mut self, name: S, _method: M)
    where
        T: 'static,
        M: Fn(Lua, mlua::UserDataRefMut<T>, A) -> MR + MaybeSend + 'static,
        A: FromLuaMulti + TypedMultiValue,
        MR: std::future::Future<Output = mlua::Result<R>> + MaybeSend + 'static,
        R: IntoLuaMulti + TypedMultiValue,
    {
        let name: Cow<'static, str> = name.into().into();
        self.raw.methods.insert(
            name.into(),
            Func::new::<A, R>(
                self.queued_doc.take(),
                self.queued_params.drain(..).collect(),
                self.queued_returns.drain(..).collect(),
            ),
        );
    }

    fn add_function_mut<S, A, R, F>(&mut self, name: S, _: F)
    where
        S: Into<String>,
        A: FromLuaMulti + TypedMultiValue,
        R: IntoLuaMulti + TypedMultiValue,
        F: 'static + MaybeSend + FnMut(&Lua, A) -> mlua::Result<R>,
    {
        let name: Cow<'static, str> = name.into().into();
        self.raw.functions.insert(
            name.into(),
            Func::new::<A, R>(
                self.queued_doc.take(),
                self.queued_params.drain(..).collect(),
                self.queued_returns.drain(..).collect(),
            ),
        );
    }

    fn add_meta_function<A, R, F>(&mut self, meta: impl Into<String>, _: F)
    where
        A: FromLuaMulti + TypedMultiValue,
        R: IntoLuaMulti + TypedMultiValue,
        F: 'static + MaybeSend + Fn(&Lua, A) -> mlua::Result<R>,
    {
        let name: Cow<'static, str> = meta.into().into();
        self.raw.meta_functions.insert(
            name.into(),
            Func::new::<A, R>(
                self.queued_doc.take(),
                self.queued_params.drain(..).collect(),
                self.queued_returns.drain(..).collect(),
            ),
        );
    }

    #[cfg(feature = "async")]
    fn add_async_function<S, A, R, F, FR>(&mut self, name: S, _: F)
    where
        S: Into<String>,
        A: FromLuaMulti + TypedMultiValue,
        R: IntoLuaMulti + TypedMultiValue,
        F: 'static + MaybeSend + Fn(Lua, A) -> FR,
        FR: 'static + MaybeSend + std::future::Future<Output = mlua::Result<R>>,
    {
        let name: Cow<'static, str> = name.into().into();
        self.raw.functions.insert(
            name.into(),
            Func::new::<A, R>(
                self.queued_doc.take(),
                self.queued_params.drain(..).collect(),
                self.queued_returns.drain(..).collect(),
            ),
        );
    }

    fn add_meta_method_mut<A, R, M>(&mut self, meta: impl Into<String>, _: M)
    where
        A: FromLuaMulti + TypedMultiValue,
        R: IntoLuaMulti + TypedMultiValue,
        M: 'static + MaybeSend + FnMut(&Lua, &mut T, A) -> mlua::Result<R>,
    {
        let name: Cow<'static, str> = meta.into().into();
        self.raw.meta_methods.insert(
            name.into(),
            Func::new::<A, R>(
                self.queued_doc.take(),
                self.queued_params.drain(..).collect(),
                self.queued_returns.drain(..).collect(),
            ),
        );
    }

    fn add_meta_function_mut<A, R, F>(&mut self, meta: impl Into<String>, _: F)
    where
        A: FromLuaMulti + TypedMultiValue,
        R: IntoLuaMulti + TypedMultiValue,
        F: 'static + MaybeSend + FnMut(&Lua, A) -> mlua::Result<R>,
    {
        let name: Cow<'static, str> = meta.into().into();
        self.raw.meta_functions.insert(
            name.into(),
            Func::new::<A, R>(
                self.queued_doc.take(),
                self.queued_params.drain(..).collect(),
                self.queued_returns.drain(..).collect(),
            ),
        );
    }
}
