use std::{borrow::Cow, collections::BTreeMap};

use mlua::{AnyUserData, FromLua, FromLuaMulti, IntoLua, IntoLuaMulti, Lua};

use crate::{
    typed::{Field, FieldAccess, Func, Index, IntoDocComment, Type},
    MaybeSend,
};

use super::{
    Typed, TypedDataDocumentation, TypedDataFields, TypedDataMethods, TypedMultiValue,
    TypedUserData,
};

/// Type information for a lua `class`. This happens to be a [`TypedUserData`]
#[derive(Default, Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct TypedClassBuilder {
    pub type_doc: Option<Cow<'static, str>>,
    queued_doc: Option<String>,
    queued_params: Vec<(String, String)>,
    queued_returns: Vec<String>,

    pub fields: BTreeMap<Index, Field>,
    pub static_fields: BTreeMap<Index, Field>,
    pub meta_fields: BTreeMap<Index, Field>,
    pub methods: BTreeMap<Index, Func>,
    pub meta_methods: BTreeMap<Index, Func>,
    pub functions: BTreeMap<Index, Func>,
    pub meta_functions: BTreeMap<Index, Func>,
}

impl From<TypedClassBuilder> for Type {
    fn from(value: TypedClassBuilder) -> Self {
        Type::Class(Box::new(value))
    }
}

impl TypedClassBuilder {
    pub fn new<T: TypedUserData>() -> Self {
        let mut gen = Self::default();
        T::add_documentation(&mut gen);
        T::add_fields(&mut gen);
        T::add_methods(&mut gen);
        gen
    }

    /// Check if any of there are any meta fields, functions, or methods present
    pub fn is_meta_empty(&self) -> bool {
        self.meta_fields.is_empty()
            && self.meta_functions.is_empty()
            && self.meta_methods.is_empty()
    }

    /// Creates a new typed field and adds it to the class's type information
    ///
    /// # Example
    ///
    /// ```
    /// use mlua_extras::typed::{TypedClassBuilder, Type};
    ///
    /// static NAME: &str = "mlua_extras";
    ///
    /// TypedClassBuilder::default()
    ///     .field("data1", Type::string() | Type::nil(), "doc comment goes last")
    ///     .field("data2", Type::array(Type::string()), ()) // Can also use `None` instead of `()`
    ///     .field("message", Type::string(), format!("A message for {NAME}"));
    /// ```
    pub fn field(mut self, key: impl Into<Index>, ty: Type, doc: impl IntoDocComment) -> Self {
        self.fields.insert(key.into(), Field::new(ty, doc));
        self
    }

    /// Creates a new typed function and adds it to the class's type information
    ///
    /// # Example
    ///
    /// ```
    /// use mlua_extras::typed::{TypedClassBuilder, Type};
    ///
    /// TypedClassBuilder::default()
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
        self.functions.insert(
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
    /// use mlua_extras::typed::{TypedClassBuilder, Type};
    ///
    /// TypedClassBuilder::default()
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
        self.methods.insert(
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
    /// use mlua_extras::typed::{TypedClassBuilder, Type};
    ///
    /// static NAME: &str = "mlua_extras";
    ///
    /// TypedClassBuilder::default()
    ///     .meta_field("data1", Type::string() | Type::nil(), "doc comment goes last")
    ///     .meta_field("data2", Type::array(Type::string()), ()) // Can also use `None` instead of `()`
    ///     .meta_field("message", Type::string(), format!("A message for {NAME}"));
    /// ```
    pub fn meta_field(mut self, key: impl Into<Index>, ty: Type, doc: impl IntoDocComment) -> Self {
        self.meta_fields.insert(key.into(), Field::new(ty, doc));
        self
    }

    /// Creates a new typed function and adds it to the class's meta type information
    ///
    /// # Example
    ///
    /// ```
    /// use mlua_extras::typed::{TypedClassBuilder, Type};
    ///
    /// TypedClassBuilder::default()
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
        self.meta_functions.insert(
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
    /// use mlua_extras::typed::{TypedClassBuilder, Type};
    ///
    /// static NAME: &str = "mlua_extras";
    ///
    /// TypedClassBuilder::default()
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
        self.meta_methods.insert(
            key.into(),
            Func::new::<Params, Returns>(
                doc,
                self.queued_params.drain(..).collect(),
                self.queued_returns.drain(..).collect(),
            ),
        );
        self
    }
}

impl<T: TypedUserData> TypedDataDocumentation<T> for TypedClassBuilder {
    fn add(&mut self, doc: &str) -> &mut Self {
        if let Some(type_doc) = self.type_doc.as_mut() {
            *type_doc = format!("{type_doc}\n{doc}").into()
        } else {
            self.type_doc = Some(doc.to_string().into())
        }
        self
    }
}

impl<T: TypedUserData> TypedDataFields<T> for TypedClassBuilder {
    fn document(&mut self, doc: &str) -> &mut Self {
        self.queued_doc = Some(doc.to_string());
        self
    }

    fn add_field<V>(&mut self, name: impl Into<String>, _: V)
    where
        V: IntoLua + Clone + 'static + Typed,
    {
        let name: Cow<'static, str> = name.into().into();
        self.static_fields
            .entry(name.into())
            .and_modify(|v| {
                v.doc = self.queued_doc.take().map(|v| v.into());
                v.ty = v.ty.clone() | V::ty();
                v.access = v.access.merge(FieldAccess::ReadWrite);
            })
            .or_insert(Field {
                ty: V::ty(),
                doc: self.queued_doc.take().map(|v| v.into()),
                access: FieldAccess::ReadWrite,
            });
    }

    fn add_field_function_set<S, A, F>(&mut self, name: S, _: F)
    where
        S: Into<String>,
        A: FromLua + Typed,
        F: 'static + MaybeSend + FnMut(&Lua, AnyUserData, A) -> mlua::Result<()>,
    {
        let name: Cow<'static, str> = name.into().into();
        self.static_fields
            .entry(name.into())
            .and_modify(|v| {
                v.doc = self.queued_doc.take().map(|v| v.into());
                v.ty = v.ty.clone() | A::ty();
                v.access = v.access.merge(FieldAccess::WriteOnly);
            })
            .or_insert(Field {
                ty: A::ty(),
                doc: self.queued_doc.take().map(|v| v.into()),
                access: FieldAccess::WriteOnly,
            });
    }

    fn add_field_function_get<S, R, F>(&mut self, name: S, _: F)
    where
        S: Into<String>,
        R: IntoLua + Typed,
        F: 'static + MaybeSend + Fn(&Lua, AnyUserData) -> mlua::Result<R>,
    {
        let name: Cow<'static, str> = name.into().into();
        self.static_fields
            .entry(name.into())
            .and_modify(|v| {
                v.doc = self.queued_doc.take().map(|v| v.into());
                v.ty = v.ty.clone() | R::ty();
                v.access = v.access.merge(FieldAccess::ReadOnly);
            })
            .or_insert(Field {
                ty: R::ty(),
                doc: self.queued_doc.take().map(|v| v.into()),
                access: FieldAccess::ReadOnly,
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
        self.static_fields
            .entry(name.into())
            .and_modify(|v| {
                v.doc = self.queued_doc.take().map(|v| v.into());
                v.ty = v.ty.clone() | A::ty() | R::ty();
                v.access = v.access.merge(FieldAccess::ReadWrite);
            })
            .or_insert(Field {
                ty: A::ty() | R::ty(),
                doc: self.queued_doc.take().map(|v| v.into()),
                access: FieldAccess::ReadWrite,
            });
    }

    fn add_field_method_set<S, A, M>(&mut self, name: S, _: M)
    where
        S: Into<String>,
        A: FromLua + Typed,
        M: 'static + MaybeSend + FnMut(&Lua, &mut T, A) -> mlua::Result<()>,
    {
        let name: Cow<'static, str> = name.into().into();
        self.fields
            .entry(name.into())
            .and_modify(|v| {
                v.doc = self.queued_doc.take().map(|v| v.into());
                v.ty = v.ty.clone() | A::ty();
                v.access = v.access.merge(FieldAccess::WriteOnly);
            })
            .or_insert(Field {
                ty: A::ty(),
                doc: self.queued_doc.take().map(|v| v.into()),
                access: FieldAccess::WriteOnly,
            });
    }

    fn add_field_method_get<S, R, M>(&mut self, name: S, _: M)
    where
        S: Into<String>,
        R: IntoLua + Typed,
        M: 'static + MaybeSend + Fn(&Lua, &T) -> mlua::Result<R>,
    {
        let name: Cow<'static, str> = name.into().into();
        self.fields
            .entry(name.into())
            .and_modify(|v| {
                v.doc = self.queued_doc.take().map(|v| v.into());
                v.ty = v.ty.clone() | R::ty();
                v.access = v.access.merge(FieldAccess::ReadOnly);
            })
            .or_insert(Field {
                ty: R::ty(),
                doc: self.queued_doc.take().map(|v| v.into()),
                access: FieldAccess::ReadOnly,
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
        self.fields
            .entry(name.into())
            .and_modify(|v| {
                v.doc = self.queued_doc.take().map(|v| v.into());
                v.ty = v.ty.clone() | A::ty() | R::ty();
                v.access = v.access.merge(FieldAccess::ReadWrite);
            })
            .or_insert(Field {
                ty: A::ty() | R::ty(),
                doc: self.queued_doc.take().map(|v| v.into()),
                access: FieldAccess::ReadWrite,
            });
    }

    fn add_meta_field<V>(&mut self, meta: impl Into<String>, _: V)
    where
        V: IntoLua + Typed + 'static,
    {
        let name: Cow<'static, str> = meta.into().into();
        self.meta_fields
            .entry(name.into())
            .and_modify(|v| {
                v.doc = self.queued_doc.take().map(|v| v.into());
                v.ty = v.ty.clone() | V::ty();
            })
            .or_insert(Field {
                ty: V::ty(),
                doc: self.queued_doc.take().map(|v| v.into()),
                access: FieldAccess::ReadWrite,
            });
    }

    fn add_meta_field_with<R, F>(&mut self, meta: impl Into<String>, _: F)
        where
            F: 'static + MaybeSend + Fn(&Lua) -> mlua::Result<R>,
            R: IntoLua + Typed {

        let name: Cow<'static, str> = meta.into().into();
        self.meta_fields
            .entry(name.into())
            .and_modify(|v| {
                v.doc = self.queued_doc.take().map(|v| v.into());
                v.ty = v.ty.clone() | R::ty();
            })
            .or_insert(Field {
                ty: R::ty(),
                doc: self.queued_doc.take().map(|v| v.into()),
                access: FieldAccess::ReadWrite,
            });
    }
}

impl<T: TypedUserData> TypedDataMethods<T> for TypedClassBuilder {
    fn document(&mut self, documentation: &str) -> &mut Self {
        self.queued_doc = Some(documentation.to_string());
        self
    }

    fn param<S: std::fmt::Display, D: std::fmt::Display>(&mut self, name: S, doc: D) -> &mut Self {
        self.queued_params.push((name.to_string(), doc.to_string()));
        self
    }

    fn ret<S: std::fmt::Display>(&mut self, doc: S) -> &mut Self {
        self.queued_returns.push(doc.to_string());
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
        self.methods.insert(
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
        self.functions.insert(
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
        self.methods.insert(
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
        self.meta_methods.insert(
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
        self.methods.insert(
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
        self.methods.insert(
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
        self.functions.insert(
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
        self.meta_functions.insert(
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
        self.functions.insert(
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
        self.meta_methods.insert(
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
        self.meta_functions.insert(
            name.into(),
            Func::new::<A, R>(
                self.queued_doc.take(),
                self.queued_params.drain(..).collect(),
                self.queued_returns.drain(..).collect(),
            ),
        );
    }
}
