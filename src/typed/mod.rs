mod function;
pub mod generator;

mod class;

pub use class::{
    TypedClassBuilder, TypedClass, TypedDataDocumentation, TypedDataFields, TypedDataMethods, TypedUserData,
    WrappedBuilder,
};

use std::{
    borrow::Cow, collections::{BTreeMap, BTreeSet, HashMap, HashSet}, marker::PhantomData
};
#[cfg(feature="userdata-wrappers")]
use std::{sync::{Arc, Mutex}, cell::{Cell, RefCell}, rc::Rc};

pub use function::{Param, Return, TypedFunction};

use mlua::{IntoLua, MetaMethod, UserDataRef, UserDataRefMut, Value, Variadic};

/// Represents a lua table key
///
/// Table keys can be either a string or an integer
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, strum::EnumIs)]
pub enum Index {
    Int(isize),
    Str(Cow<'static, str>),
}

impl std::fmt::Display for Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(num) => write!(f, "[{num}]"),
            Self::Str(val) => {
                if val.chars().any(|v| !v.is_alphanumeric() && v != '_') {
                    write!(f, r#"["{val}"]"#)
                } else {
                    write!(f, "{val}")
                }
            }
        }
    }
}

impl IntoLua for Index {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::prelude::LuaResult<Value> {
        match self {
            Self::Int(num) => Ok(mlua::Value::Integer(num as mlua::Integer)),
            Self::Str(val) => val.into_lua(lua),
        }
    }
}

impl From<MetaMethod> for Index {
    fn from(value: MetaMethod) -> Self {
        Self::Str(value.as_ref().to_string().into())
    }
}

impl From<Cow<'static, str>> for Index {
    fn from(value: Cow<'static, str>) -> Self {
        Self::Str(value)
    }
}

impl From<&'static str> for Index {
    fn from(value: &'static str) -> Self {
        Self::Str(value.into())
    }
}

impl From<String> for Index {
    fn from(value: String) -> Self {
        Self::Str(value.into())
    }
}

impl From<isize> for Index {
    fn from(value: isize) -> Self {
        Self::Int(value)
    }
}

/// Representation of a lua type for a rust type
#[derive(Debug, Clone, PartialEq, strum::AsRefStr, strum::EnumIs, PartialOrd, Eq, Ord, Hash)]
pub enum Type {
    /// Represents a single type. i.e. `string`, `number`, `0`, `"literal"`, `Example`, etc...
    ///
    /// # Example
    ///
    /// ```lua
    /// --- @type string
    /// --- @type number
    /// --- @type 0
    /// --- @type "literal"
    /// --- @type Example
    /// ```
    Single(Cow<'static, str>),
    /// Represents a typed value
    ///
    /// # Example
    ///
    /// ```lua
    /// --- @type {type}
    /// value = nil
    /// ```
    Value(Box<Type>),
    /// Represents a type alias
    ///
    /// # Example
    ///
    /// ```lua
    /// --- @alias MyType {type}
    /// ```
    Alias(Box<Type>),
    /// Represents a tuple type
    ///
    /// # Example
    ///
    /// ```lua
    /// --- @type [number, integer, string]
    /// ```
    Tuple(Vec<Type>),
    /// Represents a table literal
    ///
    /// # Example
    ///
    /// ```lua
    /// --- @type { name: string, age: integer, height: number }
    /// ```
    Table(BTreeMap<Index, Type>),
    /// Represents a type union
    ///
    /// # Example
    ///
    /// ```lua
    /// --- @type string | number | "literal" | Example
    /// ```
    Union(Vec<Type>),
    /// Represents an array of a single type
    ///
    /// # Example
    /// ```lua
    /// --- @type string[]
    /// ```
    Array(Box<Type>),
    /// Represents a table with set key types and value types
    ///
    /// # Example
    ///
    /// ```lua
    /// --- @type { [string]: boolean }
    /// ```
    Map(Box<Type>, Box<Type>),
    /// Represents a function with it's parameters and return types
    ///
    /// # Example
    ///
    /// ```lua
    /// --- @type fun(self: any, name: string): string
    /// ```
    Function {
        params: Vec<Param>,
        returns: Vec<Return>,
    },
    /// References a enum type.
    ///
    /// In this instance it acts like a union since that is the closest relation between rust
    /// enums and a lua type
    ///
    /// # Example
    ///
    /// ```lua
    /// --- @alias {name} {type}
    /// ---  | {type}
    /// ---  | {type}
    /// ---  | {type}
    /// ```
    Enum(Vec<Type>),
    /// Represents a class type
    ///
    /// # Example
    ///
    /// ```lua
    /// --- @class {name}
    /// --- @field name string
    /// --- @field age integer
    /// --- @field height number
    /// ```
    Class(Box<TypedClass>),
}

/// Allows to union types
///
/// # Example
///
/// ```
/// use mlua_extras::typed::Type;
///
/// Type::string() | Type::nil();
/// ```
impl<T: Into<Type>> std::ops::BitOr<T> for Type {
    type Output = Self;

    fn bitor(self, rhs: T) -> Self::Output {
        match (self, rhs.into()) {
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
    /// Create a lua type literal from a rust value. i.e. `3`, `true`, etc...
    pub fn literal<T: IntoLuaTypeLiteral>(value: T) -> Self {
        Self::Single(value.into_lua_type_literal().into())
    }

    /// Create a reference to a name of another defined type.
    ///
    /// # Example
    ///
    /// ```lua
    /// --- @class Example
    ///
    /// --- @type Example
    /// example = nil
    /// ```
    ///
    /// ```
    /// use mlua_extras::typed::Type;
    ///
    /// // This reference the type named `Example`
    /// Type::named("Example");
    /// ```
    pub fn named(value: impl Into<Cow<'static, str>>) -> Self {
        Self::Single(value.into())
    }

    /// Create a lua builtin `string` type
    pub fn string() -> Self {
        Self::Single("string".into())
    }

    /// Create a lua builtin `integer` type
    pub fn integer() -> Self {
        Self::Single("integer".into())
    }

    /// Create a lua builtin `number` type
    pub fn number() -> Self {
        Self::Single("number".into())
    }

    /// Create a lua builtin `boolean` type
    pub fn boolean() -> Self {
        Self::Single("boolean".into())
    }

    /// Create a lua builtin `nil` type
    pub fn nil() -> Self {
        Self::Single("nil".into())
    }

    /// Create a lua builtin `any` type
    pub fn any() -> Self {
        Self::Single("any".into())
    }

    /// Create a lua builtin `lightuserdata` type
    pub fn lightuserdata() -> Self {
        Self::Single("lightuserdata".into())
    }

    /// Create a lua builtin `thread` type
    pub fn thread() -> Self {
        Self::Single("thread".into())
    }

    /// Create an enum type. This is equal to an [`alias`][crate::typed::Type::Alias]
    pub fn r#enum(types: impl IntoIterator<Item = Type>) -> Self {
        Self::Enum(types.into_iter().collect())
    }

    /// Create a type that is an alias. i.e. `--- @alias {name} string`
    pub fn alias(ty: Type) -> Self {
        Self::Alias(Box::new(ty))
    }

    /// Create a type that is an array. i.e. `{ [integer]: type }`
    pub fn array(ty: Type) -> Self {
        Self::Array(Box::new(ty))
    }

    /// Create a type that is an array. i.e. `{ [integer]: type }`
    pub fn map(key: Type, value: Type) -> Self {
        Self::Map(Box::new(key), Box::new(value))
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
    pub fn class(class: TypedClass) -> Self {
        Self::Class(Box::new(class))
    }

    /// create a type that is a function. i.e. `fun(self): number`
    pub fn function<Params: TypedMultiValue, Response: TypedMultiValue>(
        params: Vec<(String, String)>,
        returns: Vec<String>,
    ) -> Self {
        Self::Function {
            params: Params::get_types_as_params()
                .into_iter()
                .enumerate()
                .map(|(i, mut v)| {
                    if let Some((name, doc)) = params.get(i) {
                        v.name(name.clone()).doc(doc.as_str());
                    }
                    v
                })
                .collect(),
            returns: Response::get_types_as_returns()
                .into_iter()
                .enumerate()
                .map(|(i, mut v)| {
                    if let Some(doc) = returns.get(i) {
                        v.doc(doc.as_str());
                    }
                    v
                })
                .collect(),
        }
    }

    /// A table that has defined entries.
    ///
    /// If the goal is a map like syntax use [`Type::Map`] or [`Type::map`] instead
    pub fn table(items: impl IntoIterator<Item = (Index, Type)>) -> Self {
        Self::Table(items.into_iter().collect())
    }
}

pub trait IntoLuaTypeLiteral {
    /// Construct the representation of the value as a lua type
    fn into_lua_type_literal(self) -> String;
}

impl IntoLuaTypeLiteral for String {
    fn into_lua_type_literal(self) -> String {
        format!("\"{self}\"")
    }
}

impl IntoLuaTypeLiteral for &String {
    fn into_lua_type_literal(self) -> String {
        format!("\"{self}\"")
    }
}

impl IntoLuaTypeLiteral for &str {
    fn into_lua_type_literal(self) -> String {
        format!("\"{self}\"")
    }
}

macro_rules! impl_type_literal {
    ($($lit: ty),* $(,)?) => {
        $(
            impl IntoLuaTypeLiteral for $lit {
                fn into_lua_type_literal(self) -> String {
                    self.to_string()
                }
            }
            impl IntoLuaTypeLiteral for &$lit {
                fn into_lua_type_literal(self) -> String {
                    self.to_string()
                }
            }
        )*
    };
}

impl_type_literal! {
    u8, u16, u32, u64, usize, u128,
    i8, i16, i32, i64, isize, i128,
    f32, f64
}
impl_type_literal! {bool}

/// Add a lua [`Type`] representation to a rust type
pub trait Typed {
    /// Get the type representation
    fn ty() -> Type;

    #[inline(always)]
    fn implicit() -> impl IntoIterator<Item = (&'static str, Type)> {
        []
    }

    /// Get the type as a function parameter
    fn as_param() -> Type {
        Self::ty()
    }

    /// Get the type as a function return
    fn as_return() -> Type {
        Self::ty()
    }
}

#[macro_export]
macro_rules! join_types {
    ($($ty:expr),+) => {
        $crate::typed::Type::union([
            $($crate::typed::Type::from($ty),)+
        ])
    };
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
                        Type::named($name)
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
                        Type::named($name)
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

    mlua::Function => "fun()",
    mlua::Table => "table",
    mlua::AnyUserData => "userdata",
    mlua::String => "string",
    mlua::Thread => "thread",
}

impl_static_typed_generic! {
    for<'a> Cow<'a, str> => "string",
}

impl Typed for mlua::Value {
    fn ty() -> Type {
        Type::Single("any".into())
    }
}

impl<T: Typed> Typed for UserDataRef<T> {
    fn ty() -> Type {
        T::ty()
    }

    fn as_param() -> Type {
        T::as_param()
    }

    fn as_return() -> Type {
        T::as_return()
    }
}

impl<T: Typed> Typed for UserDataRefMut<T> {
    fn ty() -> Type {
        T::ty()
    }

    fn as_param() -> Type {
        T::as_param()
    }

    fn as_return() -> Type {
        T::as_return()
    }
}

impl<T: Typed> Typed for Variadic<T> {
    /// ...type
    fn ty() -> Type {
        Type::any()
    }
}

impl<T: IntoLuaTypeLiteral> From<T> for Type {
    fn from(value: T) -> Self {
        Type::Single(value.into_lua_type_literal().into())
    }
}

/// {type} | nil
impl<T: Typed> Typed for Option<T> {
    fn ty() -> Type {
        T::ty() | Type::nil()
    }

    fn as_param() -> Type {
        T::as_param() | Type::nil()
    }

    fn as_return() -> Type {
        T::as_return() | Type::nil()
    }
}

impl<T: Typed> Typed for Box<T> {
    fn ty() -> Type {
        T::ty()
    }

    fn as_param() -> Type {
        T::as_param()
    }

    fn as_return() -> Type {
        T::as_return()
    }
}

impl<T: Typed> Typed for PhantomData<T> {
    fn ty() -> Type {
        T::ty()
    }

    fn as_param() -> Type {
        T::as_param()
    }

    fn as_return() -> Type {
        T::as_return()
    }
}

// Represents a lua tuple.
//
// With luaCATS tuples are represented with square brackets.
//
// # Example
//
// ```lua
// --- @type [string, integer, "literal"]
// ```
impl<const N: usize> From<[Type; N]> for Type {
    fn from(value: [Type; N]) -> Self {
        Type::Tuple(Vec::from(value))
    }
}

// Array type

impl<I: Typed, const N: usize> Typed for [I; N] {
    fn ty() -> Type {
        Type::Array(I::ty().into())
    }

    fn as_param() -> Type {
        Type::Array(I::as_param().into())
    }

    fn as_return() -> Type {
        Type::Array(I::as_return().into())
    }
}

impl<I: Typed> Typed for Vec<I> {
    fn ty() -> Type {
        Type::Array(I::ty().into())
    }

    fn as_param() -> Type {
        Type::Array(I::as_param().into())
    }

    fn as_return() -> Type {
        Type::Array(I::as_return().into())
    }
}

impl<I: Typed> Typed for &[I] {
    fn ty() -> Type {
        Type::Array(I::ty().into())
    }

    fn as_param() -> Type {
        Type::Array(I::as_param().into())
    }

    fn as_return() -> Type {
        Type::Array(I::as_return().into())
    }
}

impl<I: Typed> Typed for HashSet<I> {
    fn ty() -> Type {
        Type::Array(I::ty().into())
    }

    fn as_param() -> Type {
        Type::Array(I::as_param().into())
    }

    fn as_return() -> Type {
        Type::Array(I::as_return().into())
    }
}

impl<I: Typed> Typed for BTreeSet<I> {
    fn ty() -> Type {
        Type::Array(I::ty().into())
    }

    fn as_param() -> Type {
        Type::Array(I::as_param().into())
    }

    fn as_return() -> Type {
        Type::Array(I::as_return().into())
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

    fn as_param() -> Type {
        Type::Map(K::as_param().into(), V::as_param().into())
    }

    fn as_return() -> Type {
        Type::Map(K::as_return().into(), V::as_return().into())
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

    fn as_param() -> Type {
        Type::Map(K::as_param().into(), V::as_param().into())
    }

    fn as_return() -> Type {
        Type::Map(K::as_return().into(), V::as_return().into())
    }
}

// External Types

#[cfg(feature="userdata-wrappers")]
impl<T: Typed> Typed for Arc<T> {
    fn ty() -> Type {
        T::ty()
    }

    fn as_param() -> Type {
        T::as_param()
    }

    fn as_return() -> Type {
        T::as_return()
    }
}

#[cfg(feature="userdata-wrappers")]
impl<T: Typed> Typed for Rc<T> {
    fn ty() -> Type {
        T::ty()
    }

    fn as_param() -> Type {
        T::as_param()
    }

    fn as_return() -> Type {
        T::as_return()
    }
}

#[cfg(feature="userdata-wrappers")]
impl<T: Typed> Typed for Cell<T> {
    fn ty() -> Type {
        T::ty()
    }

    fn as_param() -> Type {
        T::as_param()
    }

    fn as_return() -> Type {
        T::as_return()
    }
}

#[cfg(feature="userdata-wrappers")]
impl<T: Typed> Typed for RefCell<T> {
    fn ty() -> Type {
        T::ty()
    }

    fn as_param() -> Type {
        T::as_param()
    }

    fn as_return() -> Type {
        T::as_return()
    }
}

#[cfg(feature="userdata-wrappers")]
impl<T: Typed> Typed for Mutex<T> {
    fn ty() -> Type {
        T::ty()
    }

    fn as_param() -> Type {
        T::as_param()
    }

    fn as_return() -> Type {
        T::as_return()
    }
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

    fn get_types_as_returns() -> Vec<Return> {
        Vec::new()
    }

    /// Gets the type representations as used for function parameters
    fn get_types_as_params() -> Vec<Param> {
        Vec::new()
    }
}

macro_rules! impl_typed_multi_value {
    () => {
        impl TypedMultiValue for () {}
    };
    ($($name:ident) +) => {
        impl<$($name,)* > TypedMultiValue for ($($name,)*)
            where $($name: Typed,)*
        {
            #[allow(unused_mut)]
            #[allow(non_snake_case)]
            fn get_types_as_params() -> Vec<Param> {
                Vec::from([
                    $(Param {
                        doc: None,
                        name: None,
                        ty: $name::as_param(),
                    },)*
                ])
            }

            #[allow(unused_mut)]
            #[allow(non_snake_case)]
            fn get_types_as_returns() -> Vec<Return> {
                Vec::from([
                    $(Return {
                        doc: None,
                        ty: $name::as_return(),
                    },)*
                ])
            }
        }
    };
}

impl<A> TypedMultiValue for A
where
    A: Typed,
{
    fn get_types_as_params() -> Vec<Param> {
        Vec::from([Param { name: None, doc: None, ty: A::as_param()}])
    }

    fn get_types_as_returns() -> Vec<Return> {
        Vec::from([Return { doc: None, ty: A::as_return() }])
    }
}

// TODO: Replace count argument with ${count($name)} when meta-variables are stable
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
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Field {
    pub ty: Type,
    pub doc: Option<Cow<'static, str>>,
}

impl Field {
    pub fn new(ty: Type, doc: impl IntoDocComment) -> Self {
        Self {
            ty,
            doc: doc.into_doc_comment(),
        }
    }
}

/// Type information for a lua `class` field
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct StaticField {
    pub inner: Field,
    pub default: Cow<'static, str>,
}

impl StaticField {
    pub fn new(ty: Type, doc: impl IntoDocComment, default: impl Into<Cow<'static, str>>) -> Self {
        Self {
            inner: Field {
                ty,
                doc: doc.into_doc_comment(),
            },
            default: default.into()
        }
    }
}

/// Type information for a lua `class` function
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Func {
    pub params: Vec<Param>,
    pub returns: Vec<Return>,
    pub doc: Option<Cow<'static, str>>,
}

impl Func {
    pub fn new<Params, Returns>(
        doc: impl IntoDocComment,
        params: Vec<(Option<Type>, String, Option<Cow<'static, str>>)>,
        returns: Vec<(Option<Type>, Option<Cow<'static, str>>)>,
    ) -> Self
    where
        Params: TypedMultiValue,
        Returns: TypedMultiValue,
    {
        Self {
            params: Params::get_types_as_params()
                .into_iter()
                .enumerate()
                .map(|(i, mut v)| {
                    if let Some((ty, name, doc)) = params.get(i) {
                        v.name(name.clone()).doc(doc.clone());
                        if let Some(t) = ty {
                            v.ty(t.clone());
                        }
                    }
                    v
                })
                .collect(),
            returns: Returns::get_types_as_returns()
                .into_iter()
                .enumerate()
                .map(|(i, mut v)| {
                    if let Some((ty, doc)) = returns.get(i) {
                        v.doc(doc.clone());
                        if let Some(t) = ty {
                            v.ty(t.clone());
                        }
                    }
                    v
                })
                .collect(),
            doc: doc.into_doc_comment(),
        }
    }
}

/// Helper that converts multiple different types into an `Option<Cow<'static, str>>`
pub trait IntoDocComment {
    fn into_doc_comment(self) -> Option<Cow<'static, str>>;
}

impl IntoDocComment for String {
    fn into_doc_comment(self) -> Option<Cow<'static, str>> {
        (!self.trim().is_empty()).then_some(self.into())
    }
}

impl IntoDocComment for &str {
    fn into_doc_comment(self) -> Option<Cow<'static, str>> {
        (!self.trim().is_empty()).then_some(self.to_string().into())
    }
}

impl IntoDocComment for () {
    fn into_doc_comment(self) -> Option<Cow<'static, str>> {
        None
    }
}

impl IntoDocComment for Option<String> {
    fn into_doc_comment(self) -> Option<Cow<'static, str>> {
        self.and_then(|v| (!v.trim().is_empty()).then_some(v.into()))
    }
}
impl IntoDocComment for Option<Cow<'static, str>> {
    fn into_doc_comment(self) -> Option<Cow<'static, str>> {
        self.and_then(|v| (!v.trim().is_empty()).then_some(v))
    }
}
