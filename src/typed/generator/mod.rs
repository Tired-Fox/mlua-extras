use std::{borrow::Cow, collections::BTreeSet, slice::{Iter, IterMut}, vec::IntoIter};

use super::{Type, Typed, TypedFunction, TypedMultiValue, TypedUserData};

#[derive(Default, Debug, Clone)]
pub struct Definition<'def> {
    name: Cow<'def, str>,
    types: BTreeSet<Type>
}

impl<'def> Definition<'def> {
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    pub fn new(name: impl Into<Cow<'def, str>>) -> Self {
        Self {
            name: name.into(),
            types: BTreeSet::default(),
        }
    }
}

pub struct DefinitionGenerator<'def> {
    definitions: Vec<Definition<'def>>,
    current: Definition<'def>,
}
impl<'def> Default for DefinitionGenerator<'def> {
    fn default() -> Self {
        Self {
            definitions: Vec::default(),
            current: Definition::new("init"),
        }
    }
}
impl<'def> DefinitionGenerator<'def> {
    pub fn define(mut self, name: impl Into<Cow<'def, str>>) -> Self {
        if !self.current.is_empty() {
           self.definitions.push(self.current);
        }

        self.current = Definition::new(name);
        self
    }

    pub fn register_function<Params: TypedMultiValue, Response: TypedMultiValue>(mut self, name: impl Into<Cow<'static, str>>, fun: TypedFunction<Params, Response>) -> Self {
        self.current.types.insert(Type::function(name, fun));
        self
    }

    pub fn register_alias(mut self, name: impl Into<Cow<'static, str>>, ty: Type) -> Self {
        self.current.types.insert(Type::alias(name, ty));
        self
    }

    pub fn register_class<T: TypedUserData>(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.current.types.insert(Type::class::<T>(name));
        self
    }

    pub fn register_module<T: TypedUserData>(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.current.types.insert(Type::module::<T>(name));
        self
    }

    pub fn finish(mut self) -> Definitions<'def> {
        if !self.current.is_empty() {
            self.definitions.push(self.current);
        }

        Definitions {
            definitions: self.definitions
        }
    }
}

#[derive(Debug, Clone)]
pub struct Definitions<'def> {
    definitions: Vec<Definition<'def>>,
}

impl<'def> Definitions<'def> {
    pub fn generate(initial: impl Into<Cow<'def, str>>) -> DefinitionGenerator<'def> {
        DefinitionGenerator {
            definitions: Vec::default(),
            current: Definition::new(initial),
        }
    }

    pub fn iter(&self) -> Iter<'_, Definition<'def>> {
        self.definitions.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, Definition<'def>> {
        self.definitions.iter_mut()
    }
}

impl<'def> IntoIterator for Definitions<'def> {
    type Item = Definition<'def>;
    type IntoIter = IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.definitions.into_iter() 
    }
}
