mod function;
pub mod generator;

use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
};

pub use function::{Param, TypedFunction};

use mlua::{
    AnyUserData, FromLua, FromLuaMulti, IntoLua, IntoLuaMulti, Lua, MetaMethod, UserData,
    UserDataFields, UserDataMethods, Variadic,
};

use crate::MaybeSend;

/// Add a lua [`Type`] representation to a rust type
pub trait Typed {
    /// Get the type representation
    fn ty() -> Type;

    /// Get the type as a function parameter
    fn as_param() -> Param {
        Param {
            name: None,
            ty: Self::ty(),
        }
    }
}

macro_rules! impl_static_typed {
    {
        $(
            $($target: ty)|*
            => $name: literal),*
            $(,)?
    } => {
        $(
            $(
                impl Typed for $target {
                    fn ty() -> Type {
                        Type::single($name)
                    }
                }
            )*
        )*
    };
}

macro_rules! impl_static_typed_generic {
    {
        $(
            $(for<$($lt: lifetime),+> $target: ty)|*
            => $name: literal),*
            $(,)?
    } => {
        $(
            $(
                impl<$($lt,)+> Typed for $target {
                    fn ty() -> Type {
                        Type::single($name)
                    }
                }
            )*
        )*
    };
}

impl_static_typed! {
    mlua::LightUserData => "lightuserdata",
    mlua::Error => "error",
    String | &str => "string",
    u8 | u16 | u32 | u64 | usize | u128 | i8 | i16 | i32 | i64 | isize | i128 => "integer",
    f32 | f64 => "number",
    bool => "boolean",
}

impl_static_typed_generic! {
    for<'a> Cow<'a, str> => "string",
    for<'lua> mlua::Function<'lua> => "fun()",
    for<'lua> mlua::AnyUserData<'lua> => "userdata",
    for<'lua> mlua::String<'lua> => "string",
    for<'lua> mlua::Thread<'lua> => "thread",
}

impl<T: Typed> Typed for Variadic<T> {
    /// ...type
    fn ty() -> Type {
        Type::Variadic(T::ty().into())
    }

    /// @param ... type
    fn as_param() -> Param {
        Param {
            name: Some("...".into()),
            ty: T::ty(),
        }
    }
}

/// {type} | nil
impl<T: Typed> Typed for Option<T> {
    fn ty() -> Type {
        Type::Union(vec![T::ty(), Type::Single("nil".into())])
    }
}

impl From<&'static str> for Type {
    fn from(value: &'static str) -> Self {
        Type::Single(value.into())
    }
}

impl From<Cow<'static, str>> for Type {
    fn from(value: Cow<'static, str>) -> Self {
        Type::Single(value.clone())
    }
}

impl From<String> for Type {
    fn from(value: String) -> Self {
        Type::Single(value.into())
    }
}

// Array type

impl<I: Typed, const N: usize> Typed for [I; N] {
    fn ty() -> Type {
        Type::Array(I::ty().into())
    }
}
impl<I: Typed> Typed for Vec<I> {
    fn ty() -> Type {
        Type::Array(I::ty().into())
    }
}
impl<I: Typed> Typed for &[I] {
    fn ty() -> Type {
        Type::Array(I::ty().into())
    }
}
impl<I: Typed> Typed for HashSet<I> {
    fn ty() -> Type {
        Type::Array(I::ty().into())
    }
}
impl<I: Typed> Typed for BTreeSet<I> {
    fn ty() -> Type {
        Type::Array(I::ty().into())
    }
}

// Map type

impl<K, V> Typed for BTreeMap<K, V>
where
    K: Typed,
    V: Typed,
{
    fn ty() -> Type {
        Type::Map(K::ty().into(), V::ty().into())
    }
}
impl<K, V> Typed for HashMap<K, V>
where
    K: Typed,
    V: Typed,
{
    fn ty() -> Type {
        Type::Map(K::ty().into(), V::ty().into())
    }
}

/// Typed variant of [`UserData`][mlua::UserData]
pub trait TypedUserData: Sized {
    /// Add documentation to the type itself
    #[allow(unused_variables)]
    fn add_documentation<F: TypedDataDocumentation<Self>>(docs: &mut F) {}

    ///same as [UserData::add_methods](mlua::UserData::add_methods).
    ///Refer to its documentation on how to use it.
    ///
    ///only difference is that it takes a [TypedDataMethods](crate::TypedDataMethods),
    ///which is the typed version of [UserDataMethods](mlua::UserDataMethods)
    #[allow(unused_variables)]
    fn add_methods<'lua, T: TypedDataMethods<'lua, Self>>(methods: &mut T) {}

    ///same as [UserData::add_fields](mlua::UserData::add_fields).
    ///Refer to its documentation on how to use it.
    ///
    ///only difference is that it takes a [TypedDataFields](crate::TypedDataFields),
    ///which is the typed version of [UserDataFields](mlua::UserDataFields)
    #[allow(unused_variables)]
    fn add_fields<'lua, F: TypedDataFields<'lua, Self>>(fields: &mut F) {}
}

/// Used inside of [`TypedUserData`] to add doc comments to the userdata type itself
pub trait TypedDataDocumentation<T: TypedUserData> {
    fn add(&mut self, doc: &str) -> &mut Self;
}

/// Typed variant of [`UserDataFields`][mlua::UserDataFields]
pub trait TypedDataMethods<'lua, T> {
    ///Exposes a method to lua
    fn add_method<S, A, R, M>(&mut self, name: &S, method: M)
    where
        S: ?Sized + AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        M: 'static + MaybeSend + Fn(&'lua Lua, &T, A) -> mlua::Result<R>;

    ///Exposes a method to lua that has a mutable reference to Self
    fn add_method_mut<S, A, R, M>(&mut self, name: &S, method: M)
    where
        S: ?Sized + AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        M: 'static + MaybeSend + FnMut(&'lua Lua, &mut T, A) -> mlua::Result<R>;

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

    ///Exposes a function to lua (its a method that does not take Self)
    fn add_function<S, A, R, F>(&mut self, name: &S, function: F)
    where
        S: ?Sized + AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + Fn(&'lua Lua, A) -> mlua::Result<R>;

    ///Exposes a mutable function to lua
    fn add_function_mut<S, A, R, F>(&mut self, name: &S, function: F)
    where
        S: ?Sized + AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + FnMut(&'lua Lua, A) -> mlua::Result<R>;

    #[cfg(feature = "async")]
    ///exposes an async function to lua
    fn add_async_function<S: ?Sized, A, R, F, FR>(&mut self, name: &S, function: F)
    where
        S: AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + Fn(&'lua Lua, A) -> FR,
        FR: 'lua + std::future::Future<Output = mlua::Result<R>>;

    ///Exposes a meta method to lua [http://lua-users.org/wiki/MetatableEvents](http://lua-users.org/wiki/MetatableEvents)
    fn add_meta_method<A, R, M>(&mut self, meta: MetaMethod, method: M)
    where
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        M: 'static + MaybeSend + Fn(&'lua Lua, &T, A) -> mlua::Result<R>;
    ///Exposes a meta and mutable method to lua [http://lua-users.org/wiki/MetatableEvents](http://lua-users.org/wiki/MetatableEvents)
    fn add_meta_method_mut<A, R, M>(&mut self, meta: MetaMethod, method: M)
    where
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        M: 'static + MaybeSend + FnMut(&'lua Lua, &mut T, A) -> mlua::Result<R>;
    ///Exposes a meta function to lua [http://lua-users.org/wiki/MetatableEvents](http://lua-users.org/wiki/MetatableEvents)
    fn add_meta_function<A, R, F>(&mut self, meta: MetaMethod, function: F)
    where
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + Fn(&'lua Lua, A) -> mlua::Result<R>;

    ///Exposes a meta and mutable function to lua [http://lua-users.org/wiki/MetatableEvents](http://lua-users.org/wiki/MetatableEvents)
    fn add_meta_function_mut<A, R, F>(&mut self, meta: MetaMethod, function: F)
    where
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + FnMut(&'lua Lua, A) -> mlua::Result<R>;

    ///Adds documentation to the next method/function that gets added
    fn document(&mut self, doc: &str) -> &mut Self;
}

/// Typed variant of [`UserDataMethods`][mlua::UserDataMethods]
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

    /// Typed version of [add_meta_field_with](mlua::UserDataFields::add_meta_field_with)
    fn add_meta_field_with<R, F>(&mut self, meta: MetaMethod, f: F)
    where
        F: 'static + MaybeSend + Fn(&'lua Lua) -> mlua::Result<R>,
        R: IntoLua<'lua> + Typed;
}

/// Representation of a lua type for a rust type
#[derive(Debug, Clone, PartialEq, strum::AsRefStr, PartialOrd, Eq, Ord)]
pub enum Type {
    /// string
    /// nil
    /// boolean
    /// "literal"
    /// 3
    /// ... etc
    Single(Cow<'static, str>),
    Value(Box<Type>),
    /// --- @alias {name} <type>
    Alias(Box<Type>),
    /// Same as alias but with a set name predefined
    /// --- @alias {name} <type>
    Enum(Cow<'static, str>, Vec<Type>),
    /// --- @class {name}
    /// --- @field ...
    Class(Box<Class>),
    /// { [1]: <type>, [2]: <type>, ...etc }
    Tuple(Vec<Type>),
    Struct(BTreeMap<&'static str, Type>),
    Variadic(Box<Type>),
    Union(Vec<Type>),
    Array(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Function {
        params: Vec<Param>,
        returns: Vec<Type>,
    },
}

/// Allows to union types
///
/// # Example
///
/// ```
/// use mlua_extras::typed::Type;
///
/// Type::single("string") | Type::single("nil")
/// ```
impl std::ops::BitOr for Type {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Union(mut types), Self::Union(other_types)) => {
                for ty in other_types {
                    if !types.contains(&ty) {
                        types.push(ty);
                    }
                }
                Self::Union(types)
            }
            (Self::Union(mut types), other) => {
                if !types.contains(&other) {
                    types.push(other)
                }
                Self::Union(types)
            }
            (current, other) => {
                if current == other {
                    current
                } else {
                    Self::Union(Vec::from([current, other]))
                }
            }
        }
    }
}

impl Type {
    /// Create a lua type literal for a string. i.e. `"string"`
    pub fn literal_string<T: std::fmt::Display>(value: T) -> Self {
        Self::Single(format!("\"{value}\"").into())
    }

    /// Create a lua type literal from a rust value. i.e. `3`, `true`, etc...
    pub fn literal<T: std::fmt::Display>(value: T) -> Self {
        Self::Single(value.to_string().into())
    }

    /// Create a type that has a single value. i.e. `string`, `number`, etc...
    pub fn single(value: impl Into<Cow<'static, str>>) -> Self {
        Self::Single(value.into())
    }

    /// Create an enum type. This is equal to an [`alias`][crate::typed::Type::Alias]
    pub fn r#enum(
        name: impl Into<Cow<'static, str>>,
        types: impl IntoIterator<Item = Type>,
    ) -> Self {
        Self::Enum(name.into(), types.into_iter().collect())
    }

    /// Create a type that is an alias. i.e. `--- @alias {name} string`
    pub fn alias(ty: Type) -> Self {
        Self::Alias(Box::new(ty))
    }

    /// Create a type that is variadic. i.e. `...type`
    pub fn variadic(ty: Type) -> Self {
        Self::Variadic(Box::new(ty))
    }

    /// Create a type that is an array. i.e. `{ [integer]: type }`
    pub fn array(ty: Type) -> Self {
        Self::Array(Box::new(ty))
    }

    /// Create a type that is a union. i.e. `string | integer | nil`
    pub fn union(types: impl IntoIterator<Item = Type>) -> Self {
        Self::Union(types.into_iter().collect())
    }

    /// create a type that is a tuple. i.e. `{ [1]: type, [2]: type }`
    pub fn tuple(types: impl IntoIterator<Item = Type>) -> Self {
        Self::Tuple(types.into_iter().collect())
    }

    /// create a type that is a class. i.e. `--- @class {name}`
    pub fn class<T: TypedUserData>() -> Self {
        Self::Class(Box::new(Class::new::<T>()))
    }

    /// create a type that is a function. i.e. `fun(self): number`
    pub fn function<Params: TypedMultiValue, Response: TypedMultiValue>() -> Self {
        Self::Function {
            params: Params::get_types_as_params(),
            returns: Response::get_types(),
        }
    }
}

/// Helper to create a union type
#[macro_export]
macro_rules! union {
    ($($typ: expr),*) => {
        $crate::typed::Type::Union(Vec::from([$(Type::from($typ),)*]))
    };
}

/// Typed information for a lua [`MultiValue`][mlua::MultiValue]
pub trait TypedMultiValue {
    /// Gets the types contained in this collection.
    /// Order *IS* important.
    fn get_types() -> Vec<Type> {
        Self::get_types_as_params()
            .into_iter()
            .map(|v| v.ty)
            .collect::<Vec<_>>()
    }

    /// Gets the type representations as used for function parameters
    fn get_types_as_params() -> Vec<Param>;
}

macro_rules! impl_typed_multi_value {
    () => (
        impl TypedMultiValue for () {
            #[allow(unused_mut)]
            #[allow(non_snake_case)]
            fn get_types_as_params() -> Vec<Param> {
                Vec::new()
            }
        }
    );
    ($($name:ident) +) => (
        impl<$($name,)* > TypedMultiValue for ($($name,)*)
            where $($name: Typed,)*
        {
            #[allow(unused_mut)]
            #[allow(non_snake_case)]
            fn get_types_as_params() -> Vec<Param> {
                Vec::from([
                    $($name::as_param(),)*
                ])
            }
        }
    );
}

impl<A> TypedMultiValue for A
where
    A: Typed,
{
    fn get_types_as_params() -> Vec<Param> {
        Vec::from([A::as_param()])
    }
}

impl_typed_multi_value!(A B C D E F G H I J K L M N O P);
impl_typed_multi_value!(A B C D E F G H I J K L M N O);
impl_typed_multi_value!(A B C D E F G H I J K L M N);
impl_typed_multi_value!(A B C D E F G H I J K L M);
impl_typed_multi_value!(A B C D E F G H I J K L);
impl_typed_multi_value!(A B C D E F G H I J K);
impl_typed_multi_value!(A B C D E F G H I J);
impl_typed_multi_value!(A B C D E F G H I);
impl_typed_multi_value!(A B C D E F G H);
impl_typed_multi_value!(A B C D E F G);
impl_typed_multi_value!(A B C D E F);
impl_typed_multi_value!(A B C D E);
impl_typed_multi_value!(A B C D);
impl_typed_multi_value!(A B C);
impl_typed_multi_value!(A B);
impl_typed_multi_value!(A);
impl_typed_multi_value!();

/// Type information for a lua `class` field
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct Field {
    pub ty: Type,
    pub docs: Vec<String>,
}

/// Type information for a lua `class` function
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fun {
    pub params: Vec<Param>,
    pub returns: Vec<Type>,
    pub docs: Vec<String>,
}

/// Type information for a lua `class`. This happens to be a [`TypedUserData`]
#[derive(Default, Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct Class {
    pub type_doc: Vec<String>,
    queued_docs: Vec<String>,

    pub fields: BTreeMap<Cow<'static, str>, Field>,
    pub static_fields: BTreeMap<Cow<'static, str>, Field>,
    pub meta_fields: BTreeMap<Cow<'static, str>, Field>,
    pub methods: BTreeMap<Cow<'static, str>, Fun>,
    pub meta_methods: BTreeMap<Cow<'static, str>, Fun>,
    pub functions: BTreeMap<Cow<'static, str>, Fun>,
    pub meta_functions: BTreeMap<Cow<'static, str>, Fun>,
}

impl Class {
    pub fn new<T: TypedUserData>() -> Self {
        let mut gen = Self::default();
        T::add_documentation(&mut gen);
        T::add_fields(&mut gen);
        T::add_methods(&mut gen);
        gen
    }
}

impl<T: TypedUserData> TypedDataDocumentation<T> for Class {
    fn add(&mut self, doc: &str) -> &mut Self {
        self.type_doc.push(doc.to_string());
        self
    }
}

impl<'lua, T: TypedUserData> TypedDataFields<'lua, T> for Class {
    fn document(&mut self, doc: &str) -> &mut Self {
        self.queued_docs.push(doc.to_string());
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
                v.docs.append(&mut self.queued_docs);
                v.ty = v.ty.clone() | V::ty();
            })
            .or_insert(Field {
                ty: V::ty(),
                docs: self.queued_docs.drain(..).collect::<Vec<_>>(),
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
                v.docs.append(&mut self.queued_docs);
                v.ty = v.ty.clone() | A::ty();
            })
            .or_insert(Field {
                ty: A::ty(),
                docs: self.queued_docs.drain(..).collect::<Vec<_>>(),
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
                v.docs.append(&mut self.queued_docs);
                v.ty = v.ty.clone() | R::ty();
            })
            .or_insert(Field {
                ty: R::ty(),
                docs: self.queued_docs.drain(..).collect::<Vec<_>>(),
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
                v.docs.append(&mut self.queued_docs);
                v.ty = v.ty.clone() | A::ty() | R::ty();
            })
            .or_insert(Field {
                ty: A::ty() | R::ty(),
                docs: self.queued_docs.drain(..).collect::<Vec<_>>(),
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
                v.docs.append(&mut self.queued_docs);
                v.ty = v.ty.clone() | A::ty();
            })
            .or_insert(Field {
                ty: A::ty(),
                docs: self.queued_docs.drain(..).collect::<Vec<_>>(),
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
                v.docs.append(&mut self.queued_docs);
                v.ty = v.ty.clone() | R::ty();
            })
            .or_insert(Field {
                ty: R::ty(),
                docs: self.queued_docs.drain(..).collect::<Vec<_>>(),
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
                v.docs.append(&mut self.queued_docs);
                v.ty = v.ty.clone() | A::ty() | R::ty();
            })
            .or_insert(Field {
                ty: A::ty() | R::ty(),
                docs: self.queued_docs.drain(..).collect::<Vec<_>>(),
            });
    }

    fn add_meta_field_with<R, F>(&mut self, meta: MetaMethod, _: F)
    where
        F: 'static + MaybeSend + Fn(&'lua Lua) -> mlua::Result<R>,
        R: IntoLua<'lua> + Typed,
    {
        let name: Cow<'static, str> = meta.as_ref().to_string().into();
        self.meta_fields
            .entry(name)
            .and_modify(|v| {
                v.docs.append(&mut self.queued_docs);
                v.ty = v.ty.clone() | R::ty();
            })
            .or_insert(Field {
                ty: R::ty(),
                docs: self.queued_docs.drain(..).collect::<Vec<_>>(),
            });
    }
}

impl<'lua, T: TypedUserData> TypedDataMethods<'lua, T> for Class {
    fn document(&mut self, documentation: &str) -> &mut Self {
        self.queued_docs.push(documentation.to_string());
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
            Fun {
                params: A::get_types_as_params(),
                returns: R::get_types(),
                docs: self.queued_docs.drain(..).collect::<Vec<_>>(),
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
            Fun {
                params: A::get_types_as_params(),
                returns: R::get_types(),
                docs: self.queued_docs.drain(..).collect::<Vec<_>>(),
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
            Fun {
                params: A::get_types_as_params(),
                returns: R::get_types(),
                docs: self.queued_docs.drain(..).collect::<Vec<_>>(),
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
            Fun {
                params: A::get_types_as_params(),
                returns: R::get_types(),
                docs: self.queued_docs.drain(..).collect::<Vec<_>>(),
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
            Fun {
                params: A::get_types_as_params(),
                returns: R::get_types(),
                docs: self.queued_docs.drain(..).collect::<Vec<_>>(),
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
            Fun {
                params: A::get_types_as_params(),
                returns: R::get_types(),
                docs: self.queued_docs.drain(..).collect::<Vec<_>>(),
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
            Fun {
                params: A::get_types_as_params(),
                returns: R::get_types(),
                docs: self.queued_docs.drain(..).collect::<Vec<_>>(),
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
            Fun {
                params: A::get_types_as_params(),
                returns: R::get_types(),
                docs: self.queued_docs.drain(..).collect::<Vec<_>>(),
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
            Fun {
                params: A::get_types_as_params(),
                returns: R::get_types(),
                docs: self.queued_docs.drain(..).collect::<Vec<_>>(),
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
            Fun {
                params: A::get_types_as_params(),
                returns: R::get_types(),
                docs: self.queued_docs.drain(..).collect::<Vec<_>>(),
            },
        );
    }
}

/// Wrapper around a [`UserDataFields`][mlua::UserDataFields] and [`UserDataMethods`][mlua::UserDataMethods]
/// to allow [`TypedUserData`] implementations to be used for [`UserData`][mlua::UserData]
/// implementations
pub struct WrappedGenerator<'ctx, U>(&'ctx mut U);
impl<'ctx, U> WrappedGenerator<'ctx, U> {
    pub fn new(u: &'ctx mut U) -> Self {
        WrappedGenerator(u)
    }
}

impl<'lua, 'ctx, T: UserData, U: UserDataFields<'lua, T>> TypedDataFields<'lua, T>
    for WrappedGenerator<'ctx, U>
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

    fn add_meta_field_with<R, F>(&mut self, meta: MetaMethod, f: F)
    where
        F: 'static + MaybeSend + Fn(&'lua Lua) -> mlua::Result<R>,
        R: IntoLua<'lua>,
    {
        self.0.add_meta_field_with(meta, f)
    }
}

impl<'lua, 'ctx, T: UserData, U: UserDataMethods<'lua, T>> TypedDataMethods<'lua, T>
    for WrappedGenerator<'ctx, U>
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

    fn add_function<S, A, R, F>(&mut self, name: &S, function: F)
    where
        S: ?Sized + AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + Fn(&'lua Lua, A) -> mlua::Result<R>,
    {
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

    fn add_meta_method<A, R, M>(&mut self, meta: MetaMethod, method: M)
    where
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        M: 'static + MaybeSend + Fn(&'lua Lua, &T, A) -> mlua::Result<R>,
    {
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

    fn add_function_mut<S, A, R, F>(&mut self, name: &S, function: F)
    where
        S: ?Sized + AsRef<str>,
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        F: 'static + MaybeSend + FnMut(&'lua Lua, A) -> mlua::Result<R>,
    {
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

    fn add_meta_method_mut<A, R, M>(&mut self, meta: MetaMethod, method: M)
    where
        A: FromLuaMulti<'lua> + TypedMultiValue,
        R: IntoLuaMulti<'lua> + TypedMultiValue,
        M: 'static + MaybeSend + FnMut(&'lua Lua, &mut T, A) -> mlua::Result<R>,
    {
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
}
