mod function;
pub mod generator;

mod class;
mod module;

pub use class::{
    TypedClassBuilder, TypedDataFields, TypedDataMethods, TypedUserData, WrappedBuilder,
    TypedDataDocumentation,
};
pub use module::{TypedModule, TypedModuleBuilder, TypedModuleFields, TypedModuleMethods};

use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
};

use function::Return;
pub use function::{Param, TypedFunction};

use mlua::{IntoLua, MetaMethod, Value, Variadic};

/// Add a lua [`Type`] representation to a rust type
pub trait Typed {
    /// Get the type representation
    fn ty() -> Type;

    /// Get the type as a function parameter
    fn as_param() -> Param {
        Param {
            doc: None,
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
}

impl_static_typed_generic! {
    for<'a> Cow<'a, str> => "string",
    for<'lua> mlua::Function<'lua> => "fun()",
    for<'lua> mlua::Table<'lua> => "table",
    for<'lua> mlua::AnyUserData<'lua> => "userdata",
    for<'lua> mlua::String<'lua> => "string",
    for<'lua> mlua::Thread<'lua> => "thread",
}

impl<'lua> Typed for Value<'lua> {
    fn ty() -> Type {
        Type::any()
    }
}

impl<T: Typed> Typed for Variadic<T> {
    fn ty() -> Type {
        T::ty()
    }

    fn as_param() -> Param {
        Param {
            doc: None,
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

impl<T: IntoLuaTypeLiteral> From<T> for Type {
    fn from(value: T) -> Self {
        Type::Single(value.into_lua_type_literal().into())
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
impl<const N: usize> From<[Type;N]> for Type {
    fn from(value: [Type;N]) -> Self {
        Type::Tuple(Vec::from(value))
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

/// Represents a lua table key
///
/// Table keys can be either a string or an integer
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum Index {
    Int(usize),
    Str(Cow<'static, str>),
}

impl std::fmt::Display for Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(num) => write!(f, "[{num}]"),
            Self::Str(val) => if val.chars().any(|v| !v.is_alphanumeric() && v != '_') {
                write!(f, r#"["{val}"]"#)
            } else {
                write!(f, "{val}")
            }
        }
    }
}

impl<'lua> IntoLua<'lua> for Index {
    fn into_lua(self, lua: &'lua mlua::Lua) -> mlua::prelude::LuaResult<Value<'lua>> {
        match self {
            Self::Int(num) => Ok(mlua::Value::Integer(num as mlua::Integer)),
            Self::Str(val) => val.into_lua(lua)
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

impl From<usize> for Index {
    fn from(value: usize) -> Self {
        Self::Int(value)
    }
}

/// Representation of a lua type for a rust type
#[derive(Debug, Clone, PartialEq, strum::AsRefStr, strum::EnumIs, PartialOrd, Eq, Ord)]
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
    Enum(Cow<'static, str>, Vec<Type>),
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
    Class(Box<TypedClassBuilder>),
    /// Represents a global table (module)
    ///
    /// # Example
    ///
    /// ```lua
    /// module = {
    ///     data = nil,
    ///     method = function(self) end,
    /// }
    /// ````
    ///
    /// or flattened
    ///
    /// ```lua
    /// module = {}
    /// function module:method() end
    /// module.data = nil
    /// ```
    Module(Box<TypedModuleBuilder>),
}

/// Allows to union types
///
/// # Example
///
/// ```
/// use mlua_extras::typed::Type;
///
/// Type::string() | Type::nil()
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

    /// Create a references the name of another defined type.
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
    /// // This references the type named `Example`
    /// Type::named("Example")
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
    pub fn class(class: TypedClassBuilder) -> Self {
        Self::Class(Box::new(class))
    }

    /// create a type that is a global module
    pub fn module(module: TypedModuleBuilder) -> Self {
        Self::Module(Box::new(module))
    }

    /// create a type that is a function. i.e. `fun(self): number`
    pub fn function<Params: TypedMultiValue, Response: TypedMultiValue>() -> Self {
        Self::Function {
            params: Params::get_types_as_params(),
            returns: Response::get_types()
                .into_iter()
                .map(|ty| Return { doc: None, ty })
                .collect(),
        }
    }

    /// A table that has defined entries.
    ///
    /// If the goal is a map like syntax use [`Type::Map`] or [`Type::map`] instead
    pub fn table(items: impl IntoIterator<Item=(Index, Type)>) -> Self {
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

impl_type_literal!{
    u8, u16, u32, u64, usize, u128,
    i8, i16, i32, i64, isize, i128,
    f32, f64
}
impl_type_literal!{bool}

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
        Self::get_types_as_params()
            .into_iter()
            .map(|v| Return {
                doc: None,
                ty: v.ty,
            })
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
    pub doc: Option<Cow<'static, str>>,
}

impl Field {
    pub fn new(ty: Type, doc: impl IntoDocComment) -> Self {
        Self {
            ty,
            doc: doc.into_doc_comment()
        }
    }
}

/// Type information for a lua `class` function
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Func {
    pub params: Vec<Param>,
    pub returns: Vec<Return>,
    pub doc: Option<Cow<'static, str>>,
}

impl Func {
    pub fn new<Params, Returns>(doc: impl IntoDocComment) -> Self
    where
        Params: TypedMultiValue,
        Returns: TypedMultiValue,
    {
        Self {
            params: Params::get_types_as_params(),
            returns: Returns::get_types_as_returns(),
            doc: doc.into_doc_comment()
        }
    }
}

/// Helper that converts multiple different types into an `Option<Cow<'static, str>>`
pub trait IntoDocComment {
    fn into_doc_comment(self) -> Option<Cow<'static, str>>;
}

impl IntoDocComment for String {
    fn into_doc_comment(self) -> Option<Cow<'static, str>> {
        Some(self.into())
    }
}

impl IntoDocComment for &str {
    fn into_doc_comment(self) -> Option<Cow<'static, str>> {
        Some(self.to_string().into())
    }
}

impl IntoDocComment for () {
    fn into_doc_comment(self) -> Option<Cow<'static, str>> {
        None
    }
}

impl IntoDocComment for Option<String> {
    fn into_doc_comment(self) -> Option<Cow<'static, str>> {
        self.map(|v| v.into())
    }
}
