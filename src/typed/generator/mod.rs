use std::{
    borrow::Cow,
    marker::PhantomData,
    slice::{Iter, IterMut},
    vec::IntoIter,
};

use super::{
    Param, Type, Typed, TypedMultiValue,
    function::{IntoTypedFunction, Return},
};

mod type_file;
pub use type_file::DefinitionFileGenerator;

mod luau_type_file;
pub use luau_type_file::LuauDefinitionFileGenerator;

mod luau_type_file_tests;
mod type_file_tests;

/// Representation of a type that is defined in the definition file.
///
/// This type has a name and additional documentation that can be displayed
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entry {
    pub doc: Option<String>,
    pub name: String,
    pub ty: Type,
}

impl Entry {
    /// Create a new definition entry without documentation
    pub fn new(name: impl std::fmt::Display, ty: Type) -> Self {
        Self {
            doc: None,
            name: name.to_string(),
            ty,
        }
    }

    /// Create a new definition entry with documentation
    pub fn new_with<S: std::fmt::Display>(
        name: impl std::fmt::Display,
        ty: Type,
        doc: Option<S>,
    ) -> Self {
        Self {
            doc: doc.map(|v| v.to_string()),
            name: name.to_string(),
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
    pub doc: Option<Cow<'static, str>>,
    pub params: Vec<Param>,
    pub returns: Vec<Return>,
    _m: PhantomData<fn(Params) -> Returns>,
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
            returns: Returns::get_types()
                .into_iter()
                .map(|ty| Return { doc: None, ty })
                .collect(),
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
    pub fn document(&mut self, doc: impl Into<Cow<'static, str>>) {
        self.doc = Some(doc.into());
    }

    /// Update a parameter's information given it's position in the argument list
    pub fn param(&mut self, index: usize) -> Option<&mut Param> {
        self.params.get_mut(index)
    }

    /// Update a return type's information given it's position in the return list
    pub fn ret(&mut self, index: usize) -> Option<&mut Return> {
        self.returns.get_mut(index)
    }
}

/// Builder for definition entries
#[derive(Default, Debug, Clone)]
pub struct DefinitionBuilder {
    pub entries: Vec<Entry>,
    pub queued_params: Vec<(String, String)>,
    pub queued_returns: Vec<String>,
    pub queued_doc: Option<String>,
}
impl DefinitionBuilder {
    /// Queue a doc comment for the next function call or value definition
    pub fn document(mut self, doc: impl std::fmt::Display) -> Self {
        self.queued_doc.replace(doc.to_string());
        self
    }

    /// Queue a param for the next function call
    pub fn param(mut self, name: impl std::fmt::Display, doc: impl std::fmt::Display) -> Self {
        self.queued_params.push((name.to_string(), doc.to_string()));
        self
    }

    /// Queue a return for the next function call
    pub fn ret(mut self, doc: impl std::fmt::Display) -> Self {
        self.queued_returns.push(doc.to_string());
        self
    }

    /// Register a definition entry that is a function type
    pub fn function<Params, Returns>(
        mut self,
        name: impl std::fmt::Display,
        _: impl IntoTypedFunction<Params, Returns>,
    ) -> Self
    where
        Params: TypedMultiValue,
        Returns: TypedMultiValue,
    {
        self.entries.push(Entry::new_with(
            name,
            Type::function::<Params, Returns>(
                self.queued_params.drain(..).collect(),
                self.queued_returns.drain(..).collect(),
            ),
            self.queued_doc.take(),
        ));
        self
    }

    /// Register a type
    pub fn register<T: Typed>(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        for (name, ty) in T::implicit().into_iter() {
            let ty = match ty {
                Type::Class(_) | Type::Enum(_) => ty,
                _ => Type::alias(ty),
            };
            self.entries.push(Entry::new(name, ty));
        }

        let ty = T::ty();
        let ty = match &ty {
            Type::Class(_) | Type::Enum(_) => ty,
            _ => Type::alias(ty),
        };

        self.entries.push(Entry::new(name.into(), ty.into()));
        self
    }

    /// Register a already built type.
    pub fn register_as(mut self, name: impl Into<Cow<'static, str>>, ty: impl Into<Type>) -> Self {
        let ty = ty.into();
        let ty = match &ty {
            Type::Class(_) | Type::Enum(_) => ty,
            _ => Type::alias(ty),
        };

        self.entries.push(Entry::new(name.into(), ty.into()));
        self
    }

    /// Register a value that is available
    ///
    /// This can be a table, union/enum, literal, or any other value and it will be typed
    /// with the given type
    ///
    /// # Example
    /// ```
    /// use mlua_extras::{typed::{generator::{Definitions, Definition}, TypedUserData, TypedDataFields}, Typed, UserData};
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
    ///     fn add_fields<F: TypedDataFields<Self>>(fields: &mut F) {
    ///         fields
    ///             .document("Example field")
    ///             .add_field_method_get_set(
    ///                 "color",
    ///                 |_lua, this| Ok(this.color.clone()),
    ///                 |_lua, this, clr: String| {
    ///                     this.color = clr;
    ///                     Ok(())
    ///                 },
    ///             );
    ///     }
    /// }
    ///
    /// Definitions::start()
    ///     .define("init", Definition::start()
    ///         .register::<Example>("Example")
    ///         .value::<Example>("example")
    ///     )
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
    pub fn value<T: Typed>(mut self, name: impl std::fmt::Display) -> Self {
        self.entries.push(Entry::new_with(
            name,
            Type::Value(Box::new(T::ty())),
            self.queued_doc.take(),
        ));
        self
    }

    /// Finish the definition
    pub fn finish(self) -> Definition {
        Definition {
            entries: self.entries,
        }
    }
}

/// A named group of definition entries
///
/// This is commonly represented as an individual definition file
#[derive(Default, Debug, Clone)]
pub struct Definition {
    pub entries: Vec<Entry>,
}

impl Definition {
    pub fn start() -> DefinitionBuilder {
        DefinitionBuilder::default()
    }

    /// Check if the definition grouping has any entries
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> Iter<'_, Entry> {
        self.entries.iter()
    }
}

/// Generate definition entries and definition groups
#[derive(Default)]
pub struct DefinitionsBuilder {
    definitions: Vec<(String, Definition)>,
}

impl DefinitionsBuilder {
    /// Creat a new named definition group
    pub fn define(
        mut self,
        name: impl std::fmt::Display,
        definition: impl Into<Definition>,
    ) -> Self {
        self.definitions.push((name.to_string(), definition.into()));
        self
    }

    /// Finish defining definition groups and collect them
    pub fn finish(self) -> Definitions {
        Definitions {
            definitions: self.definitions,
        }
    }
}

impl From<DefinitionBuilder> for Definition {
    fn from(value: DefinitionBuilder) -> Self {
        Definition {
            entries: value.entries,
        }
    }
}

/// A set collection of definition groups
#[derive(Default, Debug, Clone)]
pub struct Definitions {
    definitions: Vec<(String, Definition)>,
}

impl Definitions {
    /// Create a definition generator with the given name as the first definition group
    pub fn start() -> DefinitionsBuilder {
        DefinitionsBuilder {
            definitions: Vec::default(),
        }
    }

    pub fn iter(&self) -> Iter<'_, (String, Definition)> {
        self.definitions.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, (String, Definition)> {
        self.definitions.iter_mut()
    }
}

impl IntoIterator for Definitions {
    type Item = (String, Definition);
    type IntoIter = IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.definitions.into_iter()
    }
}
