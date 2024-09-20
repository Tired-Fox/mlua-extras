use std::{
    borrow::Cow,
    slice::{Iter, IterMut},
    vec::IntoIter,
};

use super::{function::IntoTypedFunction, Type, Typed, TypedMultiValue, TypedUserData};

mod type_file;
pub use type_file::TypeFileGenerator;

/// Representation of a type that is defined in the definition file.
///
/// This type has a name and additional documentation that can be displayed
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DefinitionEntry<'def> {
    pub docs: Vec<String>,
    pub name: Cow<'def, str>,
    pub ty: Type,
}

impl<'def> DefinitionEntry<'def> {
    /// Create a new definition entry without documentation
    pub fn new(name: impl Into<Cow<'def, str>>, ty: Type) -> Self {
        Self {
            docs: Vec::default(),
            name: name.into(),
            ty,
        }
    }

    /// Create a new definition entry with documentation
    pub fn new_with<S: AsRef<str>>(
        name: impl Into<Cow<'def, str>>,
        ty: Type,
        docs: impl IntoIterator<Item = S>,
    ) -> Self {
        Self {
            docs: docs.into_iter().map(|v| v.as_ref().to_string()).collect(),
            name: name.into(),
            ty,
        }
    }
}

/// A named group of definition entries
///
/// This is commonly represented as an individual definition file
#[derive(Default, Debug, Clone)]
pub struct DefinitionGroup<'def> {
    pub name: Cow<'def, str>,
    pub entries: Vec<DefinitionEntry<'def>>,
}

impl<'def> DefinitionGroup<'def> {
    /// Check if the definition grouping has any entries
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Creat a new named definition entry grouping
    pub fn new(name: impl Into<Cow<'def, str>>) -> Self {
        Self {
            name: name.into(),
            entries: Vec::default(),
        }
    }

    pub fn iter(&self) -> Iter<'def, DefinitionEntry<'_>> {
        self.entries.iter()
    }
}

/// Generate definition entries and definition groups
pub struct DefinitionGenerator<'def> {
    definitions: Vec<DefinitionGroup<'def>>,
    current: DefinitionGroup<'def>,
}
impl<'def> Default for DefinitionGenerator<'def> {
    fn default() -> Self {
        Self {
            definitions: Vec::default(),
            current: DefinitionGroup::new("init"),
        }
    }
}
impl<'def> DefinitionGenerator<'def> {
    /// Creat a new named definition group
    pub fn define(mut self, name: impl Into<Cow<'def, str>>) -> Self {
        if !self.current.is_empty() {
            self.definitions.push(self.current);
        }

        self.current = DefinitionGroup::new(name);
        self
    }

    /// Register a definition entry that is a function type
    pub fn function<'lua, Params, Response>(
        mut self,
        name: impl Into<Cow<'static, str>>,
        _: impl IntoTypedFunction<'lua, Params, Response>,
    ) -> Self
    where
        Params: TypedMultiValue,
        Response: TypedMultiValue,
    {
        self.current.entries.push(DefinitionEntry::new(
            name,
            Type::function::<Params, Response>(),
        ));
        self
    }

    /// Register a definition entry that is a function type
    ///
    /// Also add additional documentation
    pub fn function_with<'lua, Params, Response, S>(
        mut self,
        name: impl Into<Cow<'static, str>>,
        _: impl IntoTypedFunction<'lua, Params, Response>,
        docs: impl IntoIterator<Item = S>,
    ) -> Self
    where
        Params: TypedMultiValue,
        Response: TypedMultiValue,
        S: AsRef<str>,
    {
        self.current.entries.push(DefinitionEntry::new_with(
            name,
            Type::function::<Params, Response>(),
            docs,
        ));
        self
    }

    /// Register a definition entry that is an alias type
    pub fn alias(mut self, name: impl Into<Cow<'static, str>>, ty: Type) -> Self {
        self.current
            .entries
            .push(DefinitionEntry::new(name, Type::alias(ty)));
        self
    }

    /// Register a definition entry that is an alias type
    ///
    /// Also add additional documentation
    pub fn alias_with<S: AsRef<str>>(
        mut self,
        name: impl Into<Cow<'static, str>>,
        ty: Type,
        docs: impl IntoIterator<Item = S>,
    ) -> Self {
        self.current
            .entries
            .push(DefinitionEntry::new_with(name, Type::alias(ty), docs));
        self
    }

    /// Register a definition entry that is a class type
    ///
    /// The name of the class is the same as the name of the type passed
    pub fn register<T: TypedUserData>(mut self) -> Self {
        let name = std::any::type_name::<T>();
        self.current.entries.push(DefinitionEntry::new(
            name.rsplit_once("::").map(|v| v.1).unwrap_or(name),
            Type::class::<T>(),
        ));
        self
    }

    /// Same as [`register`][crate::typed::generator::DefinitionGenerator::register] but with additional docs
    pub fn register_with<T: TypedUserData, S: AsRef<str>>(
        mut self,
        docs: impl IntoIterator<Item = S>,
    ) -> Self {
        self.current.entries.push(DefinitionEntry::new_with(
            std::any::type_name::<T>(),
            Type::class::<T>(),
            docs,
        ));
        self
    }

    /// Register a definition entry that is a enum type
    ///
    /// This is equal to an alias, but is usually derived from using the `Typed` derive macro on an
    /// enum object.
    ///
    /// Returns an error response of [`Error::RuntimeError`][mlua::Error::RuntimeError] if the type extracted was not [`Type::Enum`][crate::typed::Type::Enum]
    ///
    /// # Example
    ///
    /// ```
    /// use mlua_extras::{Typed, typed::Type};
    ///
    /// #[derive(Typed)]
    /// enum Color {
    ///     Red,
    ///     White,
    ///     Green,
    ///     Yellow,
    ///     Cyan,
    ///     Blue,
    ///     Magenta,
    ///     Black
    /// }
    ///
    /// assert!(matches!(Color::ty(), Type::Enum(_, _)))
    /// ```
    pub fn register_enum<T: Typed>(mut self) -> mlua::Result<Self> {
        match T::ty() {
            Type::Enum(name, types) => {
                self.current
                    .entries
                    .push(DefinitionEntry::new(name.clone(), Type::Enum(name, types)));
            }
            other => {
                return Err(mlua::Error::runtime(format!(
                    "expected enum type was: {}",
                    other.as_ref()
                )))
            }
        }
        Ok(self)
    }

    /// Same as [`register`][crate::typed::generator::DefinitionGenerator::register_enum] but with additional docs
    pub fn register_enum_with<T: Typed, S: AsRef<str>>(
        mut self,
        docs: impl IntoIterator<Item = S>,
    ) -> mlua::Result<Self> {
        match T::ty() {
            Type::Enum(name, types) => {
                self.current.entries.push(DefinitionEntry::new_with(
                    name.clone(),
                    Type::Enum(name, types),
                    docs,
                ));
            }
            other => {
                return Err(mlua::Error::runtime(format!(
                    "expected enum type was: {}",
                    other.as_ref()
                )))
            }
        }
        Ok(self)
    }

    /// Register a value that is available
    ///
    /// This can be a table, union/enum, literal, or any other value and it will be typed
    /// with the given type
    ///
    /// # Example
    /// ```
    /// user mlua_extras::{typed::{Definitions, TypedUserData}, Typed, UserData};
    ///
    /// #[derive(UserData, Typed)]
    /// struct Example {
    ///     color: String
    /// }
    /// impl TypedUserData for Example {
    ///     fn add_documentation<F: mlua_extras::typed::TypedDataDocumentation<Self>>(docs: &mut F) {
    ///         docs.add("This is an example");
    ///     }
    ///     
    ///     fn add_fields<'lua, F: TypedDataFields<'lua, Self>>(fields: &mut F) {
    ///         fields
    ///             .document("Example field")
    ///             .add_field_method_get_set(
    ///                 "color",
    ///                 |_lua, this| Ok(this.color),
    ///                 |_lua, this, clr: String| {
    ///                     this.color = clr;
    ///                     Ok(())
    ///                 },
    ///             );
    ///     }
    /// }
    ///
    /// Definitions::generate("init")
    ///     .register::<Example>("Example")
    ///     .value::<Example>("example")
    ///     .finish();
    /// ```
    ///
    /// ```lua
    /// --- init.d.lua
    /// --- @meta
    ///
    /// --- This is an example
    /// --- @class Example
    /// --- Example field
    /// --- @field color string
    ///
    /// --- The example module
    /// --- @type Example
    /// example = nil
    /// ```
    pub fn value<T: Typed>(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.current
            .entries
            .push(DefinitionEntry::new(name, Type::Value(Box::new(T::ty()))));
        self
    }

    /// Same as [`value`][crate::typed::generator::DefinitionGenerator::value] but with additional docs
    pub fn value_with<T: Typed, S: AsRef<str>>(
        mut self,
        name: impl Into<Cow<'static, str>>,
        docs: impl IntoIterator<Item = S>,
    ) -> Self {
        self.current
            .entries
            .push(DefinitionEntry::new_with(name, Type::Value(Box::new(T::ty())), docs));
        self
    }

    pub fn finish(mut self) -> Definitions<'def> {
        if !self.current.is_empty() {
            self.definitions.push(self.current);
        }

        Definitions {
            definitions: self.definitions,
        }
    }
}

/// A set collection of definition groups
#[derive(Default, Debug, Clone)]
pub struct Definitions<'def> {
    definitions: Vec<DefinitionGroup<'def>>,
}

impl<'def> Definitions<'def> {
    /// Create a definition generator with the given name as the first definition group
    pub fn generate(initial: impl Into<Cow<'def, str>>) -> DefinitionGenerator<'def> {
        DefinitionGenerator {
            definitions: Vec::default(),
            current: DefinitionGroup::new(initial),
        }
    }

    pub fn iter(&self) -> Iter<'_, DefinitionGroup<'def>> {
        self.definitions.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, DefinitionGroup<'def>> {
        self.definitions.iter_mut()
    }
}

impl<'def> IntoIterator for Definitions<'def> {
    type Item = DefinitionGroup<'def>;
    type IntoIter = IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.definitions.into_iter()
    }
}
