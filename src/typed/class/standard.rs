use std::{any::Any, borrow::Cow, collections::BTreeMap};

use mlua::{AnyUserData, FromLua, FromLuaMulti, IntoLua, IntoLuaMulti, Lua, MetaMethod};

use crate::{typed::{function::Return, generator::FunctionBuilder, Field, Func, Index, IntoDocComment, Type}, MaybeSend};

use super::{Typed, TypedDataDocumentation, TypedDataFields, TypedDataMethods, TypedMultiValue, TypedUserData};

/// Type information for a lua `class`. This happens to be a [`TypedUserData`]
#[derive(Default, Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct TypedClassBuilder {
    pub type_doc: Option<Cow<'static, str>>,
    queued_doc: Option<String>,

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
    ///     .field("message", Type::string(), foramt!("A message for {NAME}"))
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
    ///     .function::<String, ()>("hello", ())
    /// ```
    pub fn function<Params, Returns>(mut self, key: impl Into<Index>, doc: impl IntoDocComment) -> Self
    where
        Params: TypedMultiValue,
        Returns: TypedMultiValue,
    {
        self.functions.insert(key.into(), Func::new::<Params, Returns>(doc));
        self
    }

    /// Same as [`function`][TypedClassBuilder::function] but with an extra generator function
    /// parameter.
    ///
    /// This extra parameter allows for customization of parameter names, types, and doc comments
    /// along with return types and doc comments.
    ///
    /// # Example
    ///
    /// ```
    /// use mlua_extras::typed::{TypedClassBuilder, Type};
    ///
    /// TypedClassBuilder::default()
    ///     // Can use `None` instead of `()` for specifying the doc comment
    ///     .function_with::<String, String>("getMessage", (), |func| {
    ///         func.param(0, |param| param.name("name").doc("Name to use when constructing the message"));
    ///         func.ret(0, |ret| ret.doc("Message constructed using the provided name"))
    ///     })
    /// ```
    pub fn function_with<Params, Returns, F, R>(mut self, key: impl Into<Index>, doc: impl IntoDocComment, generator: F) -> Self
    where
        Params: TypedMultiValue,
        Returns: TypedMultiValue,
        F: Fn(&mut FunctionBuilder<Params, Returns>) -> R,
        R: Any,
    {
        let mut builder = FunctionBuilder::default();
        generator(&mut builder);

        self.functions.insert(key.into(), Func {
            params: builder.params,
            returns: builder.returns,
            doc: doc.into_doc_comment()
        });
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
    ///     .method::<String, ()>("hello", ())
    /// ```
    pub fn method<Params, Returns>(mut self, key: impl Into<Index>, doc: impl IntoDocComment) -> Self
    where
        Params: TypedMultiValue,
        Returns: TypedMultiValue,
    {
        self.methods.insert(key.into(), Func::new::<Params, Returns>(doc));
        self
    }

    /// Same as [`method`][TypedClassBuilder::method] but with an extra generator function
    /// parameter.
    ///
    /// This extra parameter allows for customization of parameter names, types, and doc comments
    /// along with return types and doc comments.
    ///
    /// # Example
    ///
    /// ```
    /// use mlua_extras::typed::{TypedClassBuilder, Type};
    ///
    /// TypedClassBuilder::default()
    ///     // Can use `None` instead of `()` for specifying the doc comment
    ///     .method_with::<String, String>("getMessage", (), |func| {
    ///         func.param(0, |param| param.name("name").doc("Name to use when constructing the message"));
    ///         func.ret(0, |ret| ret.doc("Message constructed using the provided name"))
    ///     })
    /// ```
    pub fn method_with<Params, Returns, F, R>(mut self, key: impl Into<Index>, doc: impl IntoDocComment, generator: F) -> Self
    where
        Params: TypedMultiValue,
        Returns: TypedMultiValue,
        F: Fn(&mut FunctionBuilder<Params, Returns>) -> R,
        R: Any,
    {
        let mut builder = FunctionBuilder::default();
        generator(&mut builder);

        self.methods.insert(key.into(), Func {
            params: builder.params,
            returns: builder.returns,
            doc: doc.into_doc_comment()
        });
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
    ///     .meta_field("message", Type::string(), foramt!("A message for {NAME}"))
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
    ///     .meta_function::<String, ()>("hello", ())
    /// ```
    pub fn meta_function<Params, Returns>(mut self, key: impl Into<Index>, doc: impl IntoDocComment) -> Self
    where
        Params: TypedMultiValue,
        Returns: TypedMultiValue,
    {
        self.meta_functions.insert(key.into(), Func::new::<Params, Returns>(doc));
        self
    }

    /// Same as [`meta_function`][TypedClassBuilder::meta_function] but with an extra generator function
    /// parameter.
    ///
    /// This extra parameter allows for customization of parameter names, types, and doc comments
    /// along with return types and doc comments.
    ///
    /// # Example
    ///
    /// ```
    /// use mlua_extras::typed::{TypedClassBuilder, Type};
    ///
    /// TypedClassBuilder::default()
    ///     // Can use `None` instead of `()` for specifying the doc comment
    ///     .meta_function_with::<String, String>("getMessage", (), |func| {
    ///         func.param(0, |param| param.name("name").doc("Name to use when constructing the message"));
    ///         func.ret(0, |ret| ret.doc("Message constructed using the provided name"))
    ///     })
    /// ```
    pub fn meta_function_with<Params, Returns, F, R>(mut self, key: impl Into<Index>, doc: impl IntoDocComment, generator: F) -> Self
    where
        F: Fn(&mut FunctionBuilder<Params, Returns>) -> R,
        R: Any,
        Params: TypedMultiValue,
        Returns: TypedMultiValue,
    {
        let mut builder = FunctionBuilder::default();
        generator(&mut builder);

        self.meta_functions.insert(key.into(), Func {
            params: builder.params,
            returns: builder.returns,
            doc: doc.into_doc_comment()
        });
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
    ///     .method::<String, ()>("hello", ())
    /// ```
    pub fn meta_method<Params, Returns>(mut self, key: impl Into<Index>, doc: impl IntoDocComment) -> Self
    where
        Params: TypedMultiValue,
        Returns: TypedMultiValue,
    {
        self.meta_methods.insert(key.into(), Func::new::<Params, Returns>(doc));
        self
    }

    /// Same as [`meta_method`][TypedClassBuilder::meta_method] but with an extra generator function
    /// parameter.
    ///
    /// This extra parameter allows for customization of parameter names, types, and doc comments
    /// along with return types and doc comments.
    ///
    /// # Example
    ///
    /// ```
    /// use mlua_extras::typed::{TypedClassBuilder, Type};
    ///
    /// TypedClassBuilder::default()
    ///     // Can use `None` instead of `()` for specifying the doc comment
    ///     .meta_method_with::<String, String>("getMessage", (), |func| {
    ///         func.param(0, |param| param.name("name").doc("Name to use when constructing the message"));
    ///         func.ret(0, |ret| ret.doc("Message constructed using the provided name"))
    ///     })
    /// ```
    pub fn meta_method_with<Params, Returns, F, R>(mut self, key: impl Into<Index>, doc: impl IntoDocComment, generator: F) -> Self
    where
        F: Fn(&mut FunctionBuilder<Params, Returns>) -> R,
        R: Any,
        Params: TypedMultiValue,
        Returns: TypedMultiValue,
    {
        let mut builder = FunctionBuilder::default();
        generator(&mut builder);

        self.meta_methods.insert(key.into(), Func {
            params: builder.params,
            returns: builder.returns,
            doc: doc.into_doc_comment()
        });
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

impl<'lua, T: TypedUserData> TypedDataFields<'lua, T> for TypedClassBuilder {
    fn document(&mut self, doc: &str) -> &mut Self {
        self.queued_doc = Some(doc.to_string());
        self
    }

    fn add_field<V>(&mut self, name: impl AsRef<str>, _: V)
    where
        V: IntoLua<'lua> + Clone + 'static + Typed,
    {
        let name: Cow<'static, str> = name.as_ref().to_string().into();
        self.static_fields
            .entry(name.into())
            .and_modify(|v| {
                v.doc = self.queued_doc.take().map(|v| v.into());
                v.ty = v.ty.clone() | V::ty();
            })
            .or_insert(Field {
                ty: V::ty(),
                doc: self.queued_doc.take().map(|v| v.into()),
            });
    }

    fn add_field_function_set<S, A, F>(&mut self, name: &S, _: F)
    where
        S: AsRef<str> + ?Sized,
        A: FromLua<'lua> + Typed,
        F: 'static + MaybeSend + FnMut(&'lua Lua, AnyUserData<'lua>, A) -> mlua::Result<()>,
    {
        let name: Cow<'static, str> = name.as_ref().to_string().into();
        self.static_fields
            .entry(name.into())
            .and_modify(|v| {
                v.doc = self.queued_doc.take().map(|v| v.into());
                v.ty = v.ty.clone() | A::ty();
            })
            .or_insert(Field {
                ty: A::ty(),
                doc: self.queued_doc.take().map(|v| v.into()),
            });
    }

    fn add_field_function_get<S, R, F>(&mut self, name: &S, _: F)
    where
        S: AsRef<str> + ?Sized,
        R: IntoLua<'lua> + Typed,
        F: 'static + MaybeSend + Fn(&'lua Lua, AnyUserData<'lua>) -> mlua::Result<R>,
    {
        let name: Cow<'static, str> = name.as_ref().to_string().into();
        self.static_fields
            .entry(name.into())
            .and_modify(|v| {
                v.doc = self.queued_doc.take().map(|v| v.into());
                v.ty = v.ty.clone() | R::ty();
            })
            .or_insert(Field {
                ty: R::ty(),
                doc: self.queued_doc.take().map(|v| v.into()),
            });
    }

    fn add_field_function_get_set<S, R, A, GET, SET>(&mut self, name: &S, _: GET, _: SET)
    where
        S: AsRef<str> + ?Sized,
        R: IntoLua<'lua> + Typed,
        A: FromLua<'lua> + Typed,
        GET: 'static + MaybeSend + Fn(&'lua Lua, AnyUserData<'lua>) -> mlua::Result<R>,
        SET: 'static + MaybeSend + Fn(&'lua Lua, AnyUserData<'lua>, A) -> mlua::Result<()>,
    {
        let name: Cow<'static, str> = name.as_ref().to_string().into();
        self.static_fields
            .entry(name.into())
            .and_modify(|v| {
                v.doc = self.queued_doc.take().map(|v| v.into());
                v.ty = v.ty.clone() | A::ty() | R::ty();
            })
            .or_insert(Field {
                ty: A::ty() | R::ty(),
                doc: self.queued_doc.take().map(|v| v.into()),
            });
    }

    fn add_field_method_set<S, A, M>(&mut self, name: &S, _: M)
    where
        S: AsRef<str> + ?Sized,
        A: FromLua<'lua> + Typed,
        M: 'static + MaybeSend + FnMut(&'lua Lua, &mut T, A) -> mlua::Result<()>,
    {
        let name: Cow<'static, str> = name.as_ref().to_string().into();
        self.fields
            .entry(name.into())
            .and_modify(|v| {
                v.doc = self.queued_doc.take().map(|v| v.into());
                v.ty = v.ty.clone() | A::ty();
            })
            .or_insert(Field {
                ty: A::ty(),
                doc: self.queued_doc.take().map(|v| v.into()),
            });
    }

    fn add_field_method_get<S, R, M>(&mut self, name: &S, _: M)
    where
        S: AsRef<str> + ?Sized,
        R: IntoLua<'lua> + Typed,
        M: 'static + MaybeSend + Fn(&'lua Lua, &T) -> mlua::Result<R>,
    {
        let name: Cow<'static, str> = name.as_ref().to_string().into();
        self.fields
            .entry(name.into())
            .and_modify(|v| {
                v.doc = self.queued_doc.take().map(|v| v.into());
                v.ty = v.ty.clone() | R::ty();
            })
            .or_insert(Field {
                ty: R::ty(),
                doc: self.queued_doc.take().map(|v| v.into()),
            });
    }

    fn add_field_method_get_set<S, R, A, GET, SET>(&mut self, name: &S, _: GET, _: SET)
    where
        S: AsRef<str> + ?Sized,
        R: IntoLua<'lua> + Typed,
        A: FromLua<'lua> + Typed,
        GET: 'static + MaybeSend + Fn(&'lua Lua, &T) -> mlua::Result<R>,
        SET: 'static + MaybeSend + Fn(&'lua Lua, &mut T, A) -> mlua::Result<()>,
    {
        let name: Cow<'static, str> = name.as_ref().to_string().into();
        self.fields
            .entry(name.into())
            .and_modify(|v| {
                v.doc = self.queued_doc.take().map(|v| v.into());
                v.ty = v.ty.clone() | A::ty() | R::ty();
            })
            .or_insert(Field {
                ty: A::ty() | R::ty(),
                doc: self.queued_doc.take().map(|v| v.into()),
            });
    }

    fn add_meta_field<R, F>(&mut self, meta: MetaMethod, _: F)
    where
        F: 'static + MaybeSend + Fn(&'lua Lua) -> mlua::Result<R>,
        R: IntoLua<'lua> + Typed,
    {
        let name: Cow<'static, str> = meta.as_ref().to_string().into();
        self.meta_fields
            .entry(name.into())
            .and_modify(|v| {
                v.doc = self.queued_doc.take().map(|v| v.into());
                v.ty = v.ty.clone() | R::ty();
            })
            .or_insert(Field {
                ty: R::ty(),
                doc: self.queued_doc.take().map(|v| v.into()),
            });
    }
}

impl<'lua, T: TypedUserData> TypedDataMethods<'lua, T> for TypedClassBuilder {
    fn document(&mut self, documentation: &str) -> &mut Self {
        self.queued_doc = Some(documentation.to_string());
        self
    }

    fn add_method<S, A, R, M>(&mut self, name: &S, _: M)
    where
        S: ?Sized + AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        M: 'static + MaybeSend + Fn(&'lua Lua, &T, A) -> mlua::Result<R>,
    {
        let name: Cow<'static, str> = name.as_ref().to_string().into();
        self.methods.insert(
            name.into(),
            Func {
                params: A::get_types_as_params(),
                returns: R::get_types_as_returns(),
                doc: self.queued_doc.take().map(|v| v.into()),
            },
        );
    }

    fn add_method_with<S, A, R, M, G>(&mut self, name: &S, _method: M, generator: G)
        where
            S: ?Sized + AsRef<str>,
            A: FromLuaMulti<'lua> + TypedMultiValue,
            R: IntoLuaMulti<'lua> + TypedMultiValue,
            M: 'static + MaybeSend + Fn(&'lua Lua, &T, A) -> mlua::Result<R>,
            G: Fn(&mut FunctionBuilder<A, R>) {
        
        let mut builder = FunctionBuilder::<A, R>::default();
        generator(&mut builder);

        let name: Cow<'static, str> = name.as_ref().to_string().into();
        self.methods.insert(
            name.into(),
            Func {
                params: builder.params,
                returns: builder.returns,
                doc: self.queued_doc.take().map(|v| v.into()),
            },
        );
    }

    fn add_function<S, A, R, F>(&mut self, name: &S, _: F)
    where
        S: ?Sized + AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + Fn(&'lua Lua, A) -> mlua::Result<R>,
    {
        let name: Cow<'static, str> = name.as_ref().to_string().into();
        self.functions.insert(
            name.into(),
            Func {
                params: A::get_types_as_params(),
                returns: R::get_types_as_returns(),
                doc: self.queued_doc.take().map(|v| v.into()),
            },
        );
    }

    fn add_function_with<S, A, R, F, G>(&mut self, name: &S, _function: F, generator: G)
        where
            S: ?Sized + AsRef<str>,
            A: FromLuaMulti<'lua> + TypedMultiValue,
            R: IntoLuaMulti<'lua> + TypedMultiValue,
            F: 'static + MaybeSend + Fn(&'lua Lua, A) -> mlua::Result<R>,
            G: Fn(&mut FunctionBuilder<A, R>) {
        
        let mut builder = FunctionBuilder::<A, R>::default();
        generator(&mut builder);

        let name: Cow<'static, str> = name.as_ref().to_string().into();
        self.functions.insert(
            name.into(),
            Func {
                params: builder.params,
                returns: builder.returns,
                doc: self.queued_doc.take().map(|v| v.into()),
            },
        );
    }

    fn add_method_mut<S, A, R, M>(&mut self, name: &S, _: M)
    where
        S: ?Sized + AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        M: 'static + MaybeSend + FnMut(&'lua Lua, &mut T, A) -> mlua::Result<R>,
    {
        let name: Cow<'static, str> = name.as_ref().to_string().into();
        self.methods.insert(
            name.into(),
            Func {
                params: A::get_types_as_params(),
                returns: R::get_types_as_returns(),
                doc: self.queued_doc.take().map(|v| v.into()),
            },
        );
    }

    fn add_method_mut_with<S, A, R, M, G>(&mut self, name: &S, _method: M, generator: G)
        where
            S: ?Sized + AsRef<str>,
            A: FromLuaMulti<'lua> + TypedMultiValue,
            R: IntoLuaMulti<'lua> + TypedMultiValue,
            M: 'static + MaybeSend + FnMut(&'lua Lua, &mut T, A) -> mlua::Result<R>,
            G: Fn(&mut FunctionBuilder<A, R>) {
        
        let mut builder = FunctionBuilder::<A, R>::default();
        generator(&mut builder);

        let name: Cow<'static, str> = name.as_ref().to_string().into();
        self.methods.insert(
            name.into(),
            Func {
                params: builder.params,
                returns: builder.returns,
                doc: self.queued_doc.take().map(|v| v.into()),
            },
        );
    }

    fn add_meta_method<A, R, M>(&mut self, meta: MetaMethod, _: M)
    where
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        M: 'static + MaybeSend + Fn(&'lua Lua, &T, A) -> mlua::Result<R>,
    {
        let name: Cow<'static, str> = meta.as_ref().to_string().into();
        self.meta_methods.insert(
            name.into(),
            Func {
                params: A::get_types_as_params(),
                returns: R::get_types_as_returns(),
                doc: self.queued_doc.take().map(|v| v.into()),
            },
        );
    }

    fn add_meta_method_with<A, R, M, G>(&mut self, meta: MetaMethod, _method: M, generator: G)
        where
            A: FromLuaMulti<'lua> + TypedMultiValue,
            R: IntoLuaMulti<'lua> + TypedMultiValue,
            M: 'static + MaybeSend + Fn(&'lua Lua, &T, A) -> mlua::Result<R>,
            G: Fn(&mut FunctionBuilder<A, R>) {
        
        let mut builder = FunctionBuilder::<A, R>::default();
        generator(&mut builder);

        let name: Cow<'static, str> = meta.as_ref().to_string().into();
        self.meta_methods.insert(
            name.into(),
            Func {
                params: builder.params,
                returns: builder.returns,
                doc: self.queued_doc.take().map(|v| v.into()),
            },
        );
    }

    #[cfg(feature = "async")]
    fn add_async_method<'s, S: ?Sized + AsRef<str>, A, R, M, MR>(&mut self, name: &S, _: M)
    where
        'lua: 's,
        T: 'static,
        M: Fn(&'lua Lua, &'s T, A) -> MR + MaybeSend + 'static,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        MR: std::future::Future<Output = mlua::Result<R>> + 's,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
    {
        let name: Cow<'static, str> = name.as_ref().to_string().into();
        self.methods.insert(
            name.into(),
            Func {
                params: A::get_types_as_params(),
                returns: R::get_types_as_returns(),
                doc: self.queued_doc.take().map(|v| v.into()),
            },
        );
    }

    #[cfg(feature = "async")]
    fn add_async_method_with<'s, S: ?Sized + AsRef<str>, A, R, M, MR, G>(&mut self, name: &S, _method: M, generator: G)
        where
            'lua: 's,
            T: 'static,
            M: Fn(&'lua Lua, &'s T, A) -> MR + MaybeSend + 'static,
            A: FromLuaMulti<'lua> + TypedMultiValue,
            MR: std::future::Future<Output = mlua::Result<R>> + 's,
            R: IntoLuaMulti<'lua> + TypedMultiValue,
            G: Fn(&mut FunctionBuilder<A, R>) {
        
        let mut builder = FunctionBuilder::<A, R>::default();
        generator(&mut builder);

        let name: Cow<'static, str> = name.as_ref().to_string().into();
        self.methods.insert(
            name.into(),
            Func {
                params: builder.params,
                returns: builder.returns,
                doc: self.queued_doc.take().map(|v| v.into()),
            },
        );
    }

    #[cfg(feature = "async")]
    fn add_async_method_mut<'s, S: ?Sized + AsRef<str>, A, R, M, MR>(&mut self, name: &S, method: M)
        where
            'lua: 's,
            T: 'static,
            M: Fn(&'lua Lua, &'s mut T, A) -> MR + MaybeSend + 'static,
            A: FromLuaMulti<'lua> + TypedMultiValue,
            MR: std::future::Future<Output = mlua::Result<R>> + 's,
            R: IntoLuaMulti<'lua> + TypedMultiValue {
        
        let name: Cow<'static, str> = name.as_ref().to_string().into();
        self.methods.insert(
            name.into(),
            Func {
                params: A::get_types_as_params(),
                returns: R::get_types_as_returns(),
                doc: self.queued_doc.take().map(|v| v.into()),
            },
        );
    }

    #[cfg(feature = "async")]
    fn add_async_method_mut_with<'s, S: ?Sized + AsRef<str>, A, R, M, MR, G>(&mut self, name: &S, _method: M, generator: G)
        where
            'lua: 's,
            T: 'static,
            M: Fn(&'lua Lua, &'s mut T, A) -> MR + MaybeSend + 'static,
            A: FromLuaMulti<'lua> + TypedMultiValue,
            MR: std::future::Future<Output = mlua::Result<R>> + 's,
            R: IntoLuaMulti<'lua> + TypedMultiValue,
            G: Fn(&mut FunctionBuilder<A, R>) {
        
        let mut builder = FunctionBuilder::<A, R>::default();
        generator(&mut builder);

        let name: Cow<'static, str> = name.as_ref().to_string().into();
        self.methods.insert(
            name.into(),
            Func {
                params: builder.params,
                returns: builder.returns,
                doc: self.queued_doc.take().map(|v| v.into()),
            },
        );
    }

    fn add_function_mut<S, A, R, F>(&mut self, name: &S, _: F)
    where
        S: ?Sized + AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + FnMut(&'lua Lua, A) -> mlua::Result<R>,
    {
        let name: Cow<'static, str> = name.as_ref().to_string().into();
        self.functions.insert(
            name.into(),
            Func {
                params: A::get_types_as_params(),
                returns: R::get_types_as_returns(),
                doc: self.queued_doc.take().map(|v| v.into()),
            },
        );
    }

    fn add_function_mut_with<S, A, R, F, G>(&mut self, name: &S, _function: F, generator: G)
        where
            S: ?Sized + AsRef<str>,
            A: FromLuaMulti<'lua> + TypedMultiValue,
            R: IntoLuaMulti<'lua> + TypedMultiValue,
            F: 'static + MaybeSend + FnMut(&'lua Lua, A) -> mlua::Result<R>,
            G: Fn(&mut FunctionBuilder<A, R>) {
        
        let mut builder = FunctionBuilder::<A, R>::default();
        generator(&mut builder);

        let name: Cow<'static, str> = name.as_ref().to_string().into();
        self.functions.insert(
            name.into(),
            Func {
                params: builder.params,
                returns: builder.returns,
                doc: self.queued_doc.take().map(|v| v.into()),
            },
        );
    }

    fn add_meta_function<A, R, F>(&mut self, meta: MetaMethod, _: F)
    where
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + Fn(&'lua Lua, A) -> mlua::Result<R>,
    {
        let name: Cow<'static, str> = meta.as_ref().to_string().into();
        self.meta_functions.insert(
            name.into(),
            Func {
                params: A::get_types_as_params(),
                returns: R::get_types_as_returns(),
                doc: self.queued_doc.take().map(|v| v.into()),
            },
        );
    }

    fn add_meta_function_with<A, R, F, G>(&mut self, meta: MetaMethod, _function: F, generator: G)
        where
            A: FromLuaMulti<'lua> + TypedMultiValue,
            R: IntoLuaMulti<'lua> + TypedMultiValue,
            F: 'static + MaybeSend + Fn(&'lua Lua, A) -> mlua::Result<R>,
            G: Fn(&mut FunctionBuilder<A, R>) {
        
        let mut builder = FunctionBuilder::<A, R>::default();
        generator(&mut builder);

        let name: Cow<'static, str> = meta.as_ref().to_string().into();
        self.functions.insert(
            name.into(),
            Func {
                params: builder.params,
                returns: builder.returns,
                doc: self.queued_doc.take().map(|v| v.into()),
            },
        );
    }

    #[cfg(feature = "async")]
    fn add_async_function<S: ?Sized, A, R, F, FR>(&mut self, name: &S, _: F)
    where
        S: AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + Fn(&'lua Lua, A) -> FR,
        FR: 'lua + std::future::Future<Output = mlua::Result<R>>,
    {
        let name: Cow<'static, str> = name.as_ref().to_string().into();
        self.functions.insert(
            name.into(),
            Func {
                params: A::get_types_as_params(),
                returns: R::get_types_as_returns(),
                doc: self.queued_doc.take().map(|v| v.into()),
            },
        );
    }

    #[cfg(feature = "async")]
    fn add_async_function_with<S: ?Sized, A, R, F, FR, G>(&mut self, name: &S, _function: F, generator: G)
        where
            S: AsRef<str>,
            A: FromLuaMulti<'lua> + TypedMultiValue,
            R: IntoLuaMulti<'lua> + TypedMultiValue,
            F: 'static + MaybeSend + Fn(&'lua Lua, A) -> FR,
            FR: 'lua + std::future::Future<Output = mlua::Result<R>>,
            G: Fn(&mut FunctionBuilder<A, R>) {
        
        let mut builder = FunctionBuilder::<A, R>::default();
        generator(&mut builder);

        let name: Cow<'static, str> = name.as_ref().to_string().into();
        self.functions.insert(
            name.into(),
            Func {
                params: builder.params,
                returns: builder.returns,
                doc: self.queued_doc.take().map(|v| v.into()),
            },
        );
    }

    fn add_meta_method_mut<A, R, M>(&mut self, meta: MetaMethod, _: M)
    where
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        M: 'static + MaybeSend + FnMut(&'lua Lua, &mut T, A) -> mlua::Result<R>,
    {
        let name: Cow<'static, str> = meta.as_ref().to_string().into();
        self.meta_methods.insert(
            name.into(),
            Func {
                params: A::get_types_as_params(),
                returns: R::get_types_as_returns(),
                doc: self.queued_doc.take().map(|v| v.into()),
            },
        );
    }
    
    fn add_meta_method_mut_with<A, R, M, G>(&mut self, meta: MetaMethod, _method: M, generator: G)
        where
            A: FromLuaMulti<'lua> + TypedMultiValue,
            R: IntoLuaMulti<'lua> + TypedMultiValue,
            M: 'static + MaybeSend + FnMut(&'lua Lua, &mut T, A) -> mlua::Result<R>,
            G: Fn(&mut FunctionBuilder<A, R>) {
        
        let mut builder = FunctionBuilder::<A, R>::default();
        generator(&mut builder);

        let name: Cow<'static, str> = meta.as_ref().to_string().into();
        self.meta_methods.insert(
            name.into(),
            Func {
                params: builder.params,
                returns: builder.returns,
                doc: self.queued_doc.take().map(|v| v.into()),
            },
        );
    }

    fn add_meta_function_mut<A, R, F>(&mut self, meta: MetaMethod, _: F)
    where
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + FnMut(&'lua Lua, A) -> mlua::Result<R>,
    {
        let name: Cow<'static, str> = meta.as_ref().to_string().into();
        self.meta_functions.insert(
            name.into(),
            Func {
                params: A::get_types_as_params(),
                returns: R::get_types().into_iter().map(|ty| Return { doc: None, ty }).collect(),
                doc: self.queued_doc.take().map(|v| v.into()),
            },
        );
    }

    fn add_meta_function_mut_with<A, R, F, G>(&mut self, meta: MetaMethod, _function: F, generator: G)
        where
            A: FromLuaMulti<'lua> + TypedMultiValue,
            R: IntoLuaMulti<'lua> + TypedMultiValue,
            F: 'static + MaybeSend + FnMut(&'lua Lua, A) -> mlua::Result<R>,
            G: Fn(&mut FunctionBuilder<A, R>) {
        
        let mut builder = FunctionBuilder::<A, R>::default();
        generator(&mut builder);

        let name: Cow<'static, str> = meta.as_ref().to_string().into();
        self.meta_functions.insert(
            name.into(),
            Func {
                params: builder.params,
                returns: builder.returns,
                doc: self.queued_doc.take().map(|v| v.into()),
            },
        );
    }
}
