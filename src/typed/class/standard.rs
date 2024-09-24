use std::{borrow::Cow, collections::BTreeMap};

use mlua::{AnyUserData, FromLua, FromLuaMulti, IntoLua, IntoLuaMulti, Lua, MetaMethod};

use crate::{typed::{function::Return, generator::FunctionBuilder, Func, Field}, MaybeSend};

use super::{Typed, TypedDataDocumentation, TypedDataFields, TypedDataMethods, TypedMultiValue, TypedUserData};

/// Type information for a lua `class`. This happens to be a [`TypedUserData`]
#[derive(Default, Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct TypedClassBuilder {
    pub type_doc: Option<Cow<'static, str>>,
    queued_doc: Option<String>,

    pub fields: BTreeMap<Cow<'static, str>, Field>,
    pub static_fields: BTreeMap<Cow<'static, str>, Field>,
    pub meta_fields: BTreeMap<Cow<'static, str>, Field>,
    pub methods: BTreeMap<Cow<'static, str>, Func>,
    pub meta_methods: BTreeMap<Cow<'static, str>, Func>,
    pub functions: BTreeMap<Cow<'static, str>, Func>,
    pub meta_functions: BTreeMap<Cow<'static, str>, Func>,
}

impl TypedClassBuilder {
    pub fn new<T: TypedUserData>() -> Self {
        let mut gen = Self::default();
        T::add_documentation(&mut gen);
        T::add_fields(&mut gen);
        T::add_methods(&mut gen);
        gen
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
            .entry(name)
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
            .entry(name)
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
            .entry(name)
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
            .entry(name)
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
            .entry(name)
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
            .entry(name)
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
            .entry(name)
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
            .entry(name)
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
            name,
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
            name,
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
            name,
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
            name,
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
            name,
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
            name,
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
            name,
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
            name,
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
            name,
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
            name,
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
            name,
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
            name,
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
            name,
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
            name,
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
            name,
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
            name,
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
            name,
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
            name,
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
            name,
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
            name,
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
            name,
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
            name,
            Func {
                params: builder.params,
                returns: builder.returns,
                doc: self.queued_doc.take().map(|v| v.into()),
            },
        );
    }
}
