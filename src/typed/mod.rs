mod function;
pub mod generator;

mod class;
mod module;

pub use class::{
    TypedClassBuilder, TypedDataFields, TypedDataMethods, TypedUserData, WrappedBuilder,
};
pub use module::{TypedModule, TypedModuleBuilder, TypedModuleFields, TypedModuleMethods};

use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
};

use function::Return;
pub use function::{Param, TypedFunction};

use mlua::Variadic;

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
    Class(Box<TypedClassBuilder>),
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
    /// { [1]: <type>, [2]: <type>, ...etc }
    Tuple(Vec<Type>),
    Struct(BTreeMap<&'static str, Type>),
    Variadic(Box<Type>),
    Union(Vec<Type>),
    Array(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Function {
        params: Vec<Param>,
        returns: Vec<Return>,
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
}

/// Helper to create a union type
///
/// :NOTE: This is a work in progress macro
///
/// # Example
///
/// ```
/// use mlua_extras::{union, typed::Type};
/// union!("string", "number", "nil", Type::array(Type::single("string")))
/// ```
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

/// Type information for a lua `class` function
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Func {
    pub params: Vec<Param>,
    pub returns: Vec<Return>,
    pub doc: Option<Cow<'static, str>>,
}
