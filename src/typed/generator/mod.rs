use std::{
    borrow::Cow, marker::PhantomData, slice::{Iter, IterMut}, vec::IntoIter
};

use super::{function::{IntoTypedFunction, Return}, Class, Param, Type, Typed, TypedMultiValue, TypedUserData};

mod type_file;
pub use type_file::DefinitionFileGenerator;

/// Representation of a type that is defined in the definition file.
///
/// This type has a name and additional documentation that can be displayed
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entry<'def> {
    pub doc: Option<Cow<'def, str>>,
    pub name: Cow<'def, str>,
    pub ty: Type,
}

impl<'def> Entry<'def> {
    /// Create a new definition entry without documentation
    pub fn new(name: impl Into<Cow<'def, str>>, ty: Type) -> Self {
        Self {
            doc: None,
            name: name.into(),
            ty,
        }
    }

    /// Create a new definition entry with documentation
    pub fn new_with<S: Into<Cow<'def, str>>>(
        name: impl Into<Cow<'def, str>>,
        ty: Type,
        doc: Option<S>,
    ) -> Self {
        Self {
            doc: doc.map(|v| v.into()),
            name: name.into(),
            ty,
        }
    }
}

/// Builder to add documentation to parameters and return types along with the overall function
/// type
#[derive(Debug, Clone)]
pub struct FunctionBuilder<Params, Returns>
where
    Params: TypedMultiValue,
    Returns: TypedMultiValue,
{
    doc: Option<Cow<'static, str>>,
    params: Vec<Param>,
    returns: Vec<Return>,
    _m: PhantomData<fn(Params) -> Returns>
}

impl<Params, Returns> Default for FunctionBuilder<Params, Returns>
where
    Params: TypedMultiValue,
    Returns: TypedMultiValue,
{
    fn default() -> Self {
        Self {
            doc: None,
            params: Params::get_types_as_params(),
            returns: Returns::get_types().into_iter().map(|ty| Return { doc: None, ty }).collect(),
            _m: PhantomData, 
        }        
    }
}

impl<Params, Returns> FunctionBuilder<Params, Returns>
where
    Params: TypedMultiValue,
    Returns: TypedMultiValue,
{
    /// Set the doc comment for the function type
    pub fn document(&mut self, doc: impl Into<Cow<'static, str>>) -> &mut Self {
        self.doc = Some(doc.into());
        self
    }

    /// Update a parameter's information given it's position in the argument list
    pub fn param<F>(&mut self, index: usize, generator: F) -> &mut Self
    where
        F: Fn(&mut Param)
    {
        if let Some(param) = self.params.get_mut(index) {
            generator(param);
        }
        self
    }

    /// Update a return type's information given it's position in the return list
    pub fn ret<F>(&mut self, index: usize, generator: F) -> &mut Self
    where
        F: Fn(&mut Return)
    {
        if let Some(ret) = self.returns.get_mut(index) {
            generator(ret);
        }
        self
    }
}

/// Builder for definition entries
#[derive(Default, Debug, Clone)]
pub struct DefinitionBuilder<'def> {
    pub entries: Vec<Entry<'def>>,
}
impl<'def> DefinitionBuilder<'def> {
    /// Register a definition entry that is a function type
    pub fn function<'lua, Params, Returns>(
        mut self,
        name: impl Into<Cow<'def, str>>,
        _: impl IntoTypedFunction<'lua, Params, Returns>,
    ) -> Self
    where
        Params: TypedMultiValue,
        Returns: TypedMultiValue,
    {
        self.entries
            .push(Entry::new(name, Type::function::<Params, Returns>()));
        self
    }

    /// Register a definition entry that is a function type
    ///
    /// Also add additional documentation
    pub fn function_with<'lua, Params, Returns, F>(
        mut self,
        name: impl Into<Cow<'def, str>>,
        _: impl IntoTypedFunction<'lua, Params, Returns>,
        generator: F
    ) -> Self
    where
        Params: TypedMultiValue,
        Returns: TypedMultiValue,
        F: Fn(&mut FunctionBuilder<Params, Returns>)
    {
        let mut func = FunctionBuilder::<Params, Returns>::default();
        generator(&mut func);
        self.entries.push(Entry::new_with(
            name,
            Type::Function {
                params: func.params,
                returns: func.returns
            },
            func.doc,
        ));
        self
    }

    /// Register a definition entry that is an alias type
    pub fn alias(mut self, name: impl Into<Cow<'static, str>>, ty: Type) -> Self {
        self.entries.push(Entry::new(name, Type::alias(ty)));
        self
    }

    /// Register a definition entry that is an alias type
    ///
    /// Also add additional documentation
    pub fn alias_with<S: Into<Cow<'def, str>>>(
        mut self,
        name: impl Into<Cow<'def, str>>,
        ty: Type,
        doc: Option<S>,
    ) -> Self {
        self.entries
            .push(Entry::new_with(name, Type::alias(ty), doc));
        self
    }

    /// Register a definition entry that is a class type
    ///
    /// The name of the class is the same as the name of the type passed
    pub fn register<T: TypedUserData>(mut self) -> Self {
        let name = std::any::type_name::<T>();
        self.entries.push(Entry::new(
            name.rsplit_once("::").map(|v| v.1).unwrap_or(name),
            Type::class::<T>(),
        ));
        self
    }

    /// Same as [`register`][DefinitionBuilder::register] but with additional docs
    pub fn register_with<T: TypedUserData, S: Into<Cow<'def, str>>>(
        mut self,
        doc: Option<S>,
    ) -> Self {
        self.entries.push(Entry::new_with(
            std::any::type_name::<T>(),
            Type::class::<T>(),
            doc,
        ));
        self
    }

    /// Register a definition entry that is a enum type
    ///
    /// This is equal to an alias, but is usually derived from using the `Typed` derive macro on an
    /// enum object.
    ///
    /// Returns an error response of [`Error::RuntimeError`][mlua::Error::RuntimeError] if the type extracted was not [`Type::Enum`]
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
                self.entries
                    .push(Entry::new(name.clone(), Type::Enum(name, types)));
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

    /// Same as [`register`][DefinitionBuilder::register_enum] but with additional docs
    pub fn register_enum_with<T: Typed, S: Into<Cow<'def, str>>>(
        mut self,
        doc: Option<S>,
    ) -> mlua::Result<Self> {
        match T::ty() {
            Type::Enum(name, types) => {
                self.entries
                    .push(Entry::new_with(name.clone(), Type::Enum(name, types), doc));
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
    pub fn value<T: Typed>(mut self, name: impl Into<Cow<'def, str>>) -> Self {
        self.entries
            .push(Entry::new(name, Type::Value(Box::new(T::ty()))));
        self
    }

    /// Same as [`value`][DefinitionBuilder::value] but with additional docs
    pub fn value_with<T: Typed, S: Into<Cow<'def, str>>>(
        mut self,
        name: impl Into<Cow<'def, str>>,
        doc: Option<S>,
    ) -> Self {
        self.entries
            .push(Entry::new_with(name, Type::Value(Box::new(T::ty())), doc));
        self
    }

    /// Finish the definition
    pub fn finish(self) -> Definition<'def> {
        Definition {
            entries: self.entries,
        }
    }
}

/// A named group of definition entries
///
/// This is commonly represented as an individual definition file
#[derive(Default, Debug, Clone)]
pub struct Definition<'def> {
    pub entries: Vec<Entry<'def>>,
}

impl<'def> Definition<'def> {
    pub fn start() -> DefinitionBuilder<'def> {
        DefinitionBuilder::default()
    }

    /// Check if the definition grouping has any entries
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> Iter<'def, Entry<'_>> {
        self.entries.iter()
    }
}

/// Generate definition entries and definition groups
#[derive(Default)]
pub struct DefinitionsBuilder<'def> {
    definitions: Vec<(Cow<'def, str>, Definition<'def>)>,
}

impl<'def> DefinitionsBuilder<'def> {
    /// Creat a new named definition group
    pub fn define(
        mut self,
        name: impl Into<Cow<'def, str>>,
        definition: impl Into<Definition<'def>>,
    ) -> Self {
        self.definitions.push((
            name.into(),
            definition.into(),
        ));
        self
    }

    /// Finish defining definition groups and collect them
    pub fn finish(self) -> Definitions<'def> {
        Definitions {
            definitions: self.definitions,
        }
    }
}

impl<'def> From<DefinitionBuilder<'def>> for Definition<'def> {
    fn from(value: DefinitionBuilder<'def>) -> Self {
        Definition {
            entries: value.entries,
        }
    }
}

/// A set collection of definition groups
#[derive(Default, Debug, Clone)]
pub struct Definitions<'def> {
    definitions: Vec<(Cow<'def, str>, Definition<'def>)>,
}

impl<'def> Definitions<'def> {
    /// Create a definition generator with the given name as the first definition group
    pub fn start() -> DefinitionsBuilder<'def> {
        DefinitionsBuilder {
            definitions: Vec::default(),
        }
    }

    pub fn iter(&self) -> Iter<'_, (Cow<'def, str>, Definition<'def>)> {
        self.definitions.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, (Cow<'def, str>, Definition<'def>)> {
        self.definitions.iter_mut()
    }
}

impl<'def> IntoIterator for Definitions<'def> {
    type Item = (Cow<'def, str>, Definition<'def>);
    type IntoIter = IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.definitions.into_iter()
    }
}
