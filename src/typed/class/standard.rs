use std::{borrow::Cow, collections::BTreeMap};

use mlua::{AnyUserData, FromLua, FromLuaMulti, IntoLua, IntoLuaMulti, Lua};

use crate::{
    MaybeSend,
    ser::to_lua_repr,
    typed::{Field, Func, Index, IntoDocComment, StaticField, Type},
};

use super::{
    Typed, TypedDataDocumentation, TypedDataFields, TypedDataMethods, TypedMultiValue,
    TypedUserData,
};

#[derive(Default, Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct TypedClass {
    pub type_doc: Option<Cow<'static, str>>,

    pub derives: Vec<String>,

    pub fields: BTreeMap<Index, Field>,
    pub static_fields: BTreeMap<Index, StaticField>,
    pub meta_fields: BTreeMap<Index, Field>,
    pub static_meta_fields: BTreeMap<Index, StaticField>,

    pub methods: BTreeMap<Index, Func>,
    pub meta_methods: BTreeMap<Index, Func>,

    pub functions: BTreeMap<Index, Func>,
    pub meta_functions: BTreeMap<Index, Func>,
}
impl TypedClass {
    /// Check if any of there are any meta fields, functions, or methods present
    pub fn is_meta_empty(&self) -> bool {
        self.meta_fields.is_empty()
            && self.static_meta_fields.is_empty()
            && self.meta_functions.is_empty()
            && self.meta_methods.is_empty()
    }
}

/// Type information for a lua `class`. This happens to be a [`TypedUserData`]
#[derive(Default, Debug, Clone)]
pub struct TypedClassBuilder {
    lua: Lua,

    queued_doc: Option<Cow<'static, str>>,
    queued_ty: Option<Type>,
    queued_params: Vec<(Option<Type>, String, Option<Cow<'static, str>>)>,
    queued_returns: Vec<(Option<Type>, Option<Cow<'static, str>>)>,

    pub typed_class: TypedClass,
}

impl From<TypedClassBuilder> for Type {
    fn from(value: TypedClassBuilder) -> Self {
        Type::Class(Box::new(value.typed_class))
    }
}

impl TypedClassBuilder {
    pub fn new<T: TypedUserData>() -> Self {
        let mut tcb = Self::default();
        T::add_documentation(&mut tcb);
        T::add_fields(&mut tcb);
        T::add_methods(&mut tcb);
        tcb
    }

    pub fn build(self) -> TypedClass {
        self.typed_class
    }

    /// Skip/Remove a field field from the class definition
    pub fn skip_field(mut self, idx: impl Into<Index>) -> Self {
        self.typed_class.fields.remove(&idx.into());
        self
    }

    /// Skip/Remove a method from the class definition
    pub fn skip_method(mut self, idx: impl Into<Index>) -> Self {
        self.typed_class.methods.remove(&idx.into());
        self
    }

    /// Skip/Remove a meta method from the class definition
    pub fn skip_meta_method(mut self, idx: impl Into<Index>) -> Self {
        self.typed_class.meta_methods.remove(&idx.into());
        self
    }

    /// Skip/Remove a function from the class definition
    pub fn skip_function(mut self, idx: impl Into<Index>) -> Self {
        self.typed_class.functions.remove(&idx.into());
        self
    }

    /// Skip/Remove a meta function from the class definition
    pub fn skip_meta_function(mut self, idx: impl Into<Index>) -> Self {
        self.typed_class.meta_functions.remove(&idx.into());
        self
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
        self.typed_class
            .fields
            .insert(key.into(), Field::new(ty, doc));
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
            self.typed_class
                .static_fields
                .insert(key.into(), StaticField::new(V::ty(), doc, value));
        }
        self
    }

    pub fn inherit(mut self, parent: &TypedClass) -> Self {
        self.typed_class
            .static_fields
            .extend(parent.static_fields.clone());
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
        self.typed_class.functions.insert(
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
        self.typed_class.methods.insert(
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
        self.typed_class
            .meta_fields
            .insert(key.into(), Field::new(ty, doc));
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
        self.typed_class.meta_functions.insert(
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
        self.typed_class.meta_methods.insert(
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
        self.typed_class.derives.push(parent.to_string());
        self
    }
}

impl<T: TypedUserData> TypedDataDocumentation<T> for TypedClassBuilder {
    fn add(&mut self, doc: &str) -> &mut Self {
        if let Some(type_doc) = self.typed_class.type_doc.as_mut() {
            *type_doc = format!("{type_doc}\n{doc}").into()
        } else {
            self.typed_class.type_doc = Some(doc.to_string().into())
        }
        self
    }
}

impl<T: TypedUserData> TypedDataFields<T> for TypedClassBuilder {
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

            self.typed_class.static_fields.insert(
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
        self.typed_class
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
        self.typed_class
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
        self.typed_class
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
        self.typed_class
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
        self.typed_class
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
        self.typed_class
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

            self.typed_class.static_meta_fields.insert(
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
        self.typed_class
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

impl<T: TypedUserData> TypedDataMethods<T> for TypedClassBuilder {
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
        self.typed_class.fields.insert(
            idx.into(),
            Field {
                ty: I::as_param(),
                doc: doc.into_doc_comment(),
            },
        );
        self
    }

    fn index_as(&mut self, idx: isize, ty: impl Into<Type>, doc: impl IntoDocComment) -> &mut Self {
        self.typed_class.fields.insert(
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
        self.typed_class.methods.insert(
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
        self.typed_class.functions.insert(
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
        self.typed_class.methods.insert(
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
        self.typed_class.meta_methods.insert(
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
        self.typed_class.methods.insert(
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
        self.typed_class.methods.insert(
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
        self.typed_class.functions.insert(
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
        self.typed_class.meta_functions.insert(
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
        self.typed_class.functions.insert(
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
        self.typed_class.meta_methods.insert(
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
        self.typed_class.meta_functions.insert(
            name.into(),
            Func::new::<A, R>(
                self.queued_doc.take(),
                self.queued_params.drain(..).collect(),
                self.queued_returns.drain(..).collect(),
            ),
        );
    }
}
