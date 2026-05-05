use std::{cell::RefCell, collections::HashMap, path::Path, slice::Iter};

use crate::typed::{Field, Param, StaticField, Type, function::Return};

use super::{Definition, Definitions};

/// Generates a lua definition file for each [`Definition`][`crate::typed::generator::Definition`]
///
/// Each file will start with `--- @meta` and contain types inside of doc comment to be used with
/// [LuaLsp](https://github.com/LuaLS/lua-language-server). If there are expose values those are
/// written as `{name} = nil` with a `--- @type {type}` doc comment above to mark it's value.
///
/// # Example Output
///
/// ```lua
/// --- @meta
///
/// --- @class Example
/// --- Name of the example
/// --- @field name string
/// --- Run the example returning it's success state
/// --- @field run fun(): bool
///
/// --- Global example
/// --- @type Example
/// example = nil
/// ```
pub struct DefinitionFileGenerator {
    /// Extendion of each definition file: Default [`.d.lua`]
    ///
    /// **IMPORTANT** Must start with a dot
    extension: String,
    definitions: Definitions,
}

impl Default for DefinitionFileGenerator {
    fn default() -> Self {
        Self {
            extension: ".d.lua".into(),
            definitions: Definitions::default(),
        }
    }
}

impl DefinitionFileGenerator {
    /// Create a new generator given a collection of definitions
    pub fn new(definitions: Definitions) -> Self {
        Self {
            definitions,
            ..Default::default()
        }
    }

    /// Set the extension that each file will end with
    pub fn ext(mut self, ext: impl AsRef<str>) -> Self {
        self.extension = ext.as_ref().to_string();
        self
    }

    pub fn iter(&self) -> DefinitionFileIter<'_> {
        DefinitionFileIter {
            extension: self.extension.clone(),
            definitions: self.definitions.iter(),
        }
    }
}

pub struct DefinitionFileIter<'def> {
    extension: String,
    definitions: Iter<'def, (String, Definition)>,
}

impl<'def> Iterator for DefinitionFileIter<'def> {
    type Item = (String, DefinitionWriter<'def>);

    fn next(&mut self) -> Option<Self::Item> {
        self.definitions.next().map(|v| {
            (
                format!("{}{}", v.0, self.extension),
                DefinitionWriter {
                    definition: &v.1,
                    name_map: RefCell::new(HashMap::default()),
                },
            )
        })
    }
}

pub struct DefinitionWriter<'def> {
    definition: &'def Definition,
    name_map: RefCell<HashMap<Type, String>>,
}

impl<'writer> DefinitionWriter<'writer> {
    /// Write the full definition group to a specified file
    pub fn write_file<P: AsRef<Path>>(self, path: P) -> mlua::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)?;
        self.write(&mut file)
    }

    /// Write the full definition group to the specified `io`
    pub fn write<W: std::io::Write>(self, mut buffer: W) -> mlua::Result<()> {
        writeln!(buffer, "--- @meta\n")?;

        for definition in self.definition.iter() {
            match &definition.ty {
                Type::Value(ty) => {
                    if let Some(docs) = self.accumulate_docs(&[definition.doc.as_deref()]) {
                        writeln!(buffer, "{}", docs.join("\n"))?;
                    }

                    writeln!(buffer, "--- @type {}", self.type_signature(ty)?)?;
                    writeln!(buffer, "{} = nil", definition.name)?;
                }
                Type::Class(type_data) => {
                    self.name_map
                        .borrow_mut()
                        .insert(definition.ty.clone(), definition.name.clone());

                    if let Some(docs) = self.accumulate_docs(&[
                        definition.doc.as_deref(),
                        type_data.type_doc.as_deref(),
                    ]) {
                        writeln!(buffer, "{}", docs.join("\n"))?;
                    }
                    write!(buffer, "--- @class {}", definition.name)?;
                    if !type_data.derives.is_empty() {
                        write!(buffer, ": {}", type_data.derives.join(", "))?;
                    }
                    writeln!(buffer)?;

                    for (name, field) in type_data.fields.iter() {
                        if let Some(docs) = self.accumulate_docs(&[field.doc.as_deref()]) {
                            writeln!(buffer, "{}", docs.join("\n"))?;
                        }
                        writeln!(
                            buffer,
                            "--- @field {name} {}",
                            self.type_signature(&field.ty)?
                        )?;
                    }

                    if !type_data.static_fields.is_empty()
                        || !type_data.functions.is_empty()
                        || !type_data.methods.is_empty()
                        || !type_data.is_meta_empty()
                    {
                        writeln!(buffer, "local _CLASS_{}_ = {{", definition.name)?;
                        for (
                            name,
                            StaticField {
                                inner: Field { doc, .. },
                                default,
                            },
                        ) in type_data.static_fields.iter()
                        {
                            if let Some(docs) = self.accumulate_docs(&[doc.as_deref()]) {
                                writeln!(buffer, "\t{}", docs.join("\n\t"))?;
                            }
                            writeln!(buffer, "\t{name} = {default},")?;
                        }

                        for (name, func) in type_data.functions.iter() {
                            if let Some(docs) = self.accumulate_docs(&[func.doc.as_deref()]) {
                                writeln!(buffer, "\t{}", docs.join("\n\t"))?;
                            }
                            writeln!(
                                buffer,
                                "\t{},",
                                self.function_signature(name, &func.params, &func.returns, true)?
                                    .join("\n\t")
                            )?;
                        }

                        for (name, func) in type_data.methods.iter() {
                            if let Some(docs) = self.accumulate_docs(&[func.doc.as_deref()]) {
                                writeln!(buffer, "\t{}", docs.join("\n\t"))?;
                            }
                            writeln!(
                                buffer,
                                "\t{},",
                                self.method_signature(
                                    name,
                                    definition.name.to_string(),
                                    &func.params,
                                    &func.returns,
                                    true
                                )?
                                .join("\n\t")
                            )?;
                        }

                        if !type_data.is_meta_empty() {
                            writeln!(buffer, "\t__metatable = {{")?;
                            for (
                                name,
                                StaticField {
                                    inner: Field { ty, doc },
                                    default,
                                },
                            ) in type_data.static_meta_fields.iter()
                            {
                                if let Some(docs) = self.accumulate_docs(&[doc.as_deref()]) {
                                    writeln!(buffer, "\t\t{}", docs.join("\n\t\t"))?;
                                }

                                writeln!(buffer, "\t\t--- @type {}", self.type_signature(ty)?)?;
                                writeln!(buffer, "\t\t{name} = {default},")?;
                            }

                            for (name, field) in type_data.meta_fields.iter() {
                                if let Some(docs) = self.accumulate_docs(&[field.doc.as_deref()]) {
                                    writeln!(buffer, "\t\t{}", docs.join("\n\t\t"))?;
                                }
                                writeln!(
                                    buffer,
                                    "\t\t--- @type {}",
                                    self.type_signature(&field.ty)?
                                )?;
                                writeln!(buffer, "\t\t{name} = nil,")?;
                            }

                            for (name, func) in type_data.meta_functions.iter() {
                                if let Some(docs) = self.accumulate_docs(&[func.doc.as_deref()]) {
                                    writeln!(buffer, "\t\t{}", docs.join("\n\t\t"))?;
                                }
                                writeln!(
                                    buffer,
                                    "\t\t{},",
                                    self.function_signature(
                                        name,
                                        &func.params,
                                        &func.returns,
                                        true
                                    )?
                                    .join("\n\t\t")
                                )?;
                            }

                            for (name, func) in type_data.meta_methods.iter() {
                                if let Some(docs) = self.accumulate_docs(&[func.doc.as_deref()]) {
                                    writeln!(buffer, "\t\t{}", docs.join("\n\t\t"))?;
                                }
                                writeln!(
                                    buffer,
                                    "\t\t{},",
                                    self.method_signature(
                                        name,
                                        definition.name.to_string(),
                                        &func.params,
                                        &func.returns,
                                        true
                                    )?
                                    .join("\n\t\t")
                                )?;
                            }
                            writeln!(buffer, "\t}}")?;
                        }
                        writeln!(buffer, "}}")?;
                    }
                }
                Type::Enum(types) => {
                    self.name_map
                        .borrow_mut()
                        .insert(definition.ty.clone(), definition.name.clone());

                    if let Some(docs) = self.accumulate_docs(&[definition.doc.as_deref()]) {
                        writeln!(buffer, "{}", docs.join("\n"))?;
                    }
                    writeln!(
                        buffer,
                        "--- @alias {} {}",
                        definition.name,
                        types
                            .iter()
                            .map(|v| self.type_signature(v))
                            .collect::<mlua::Result<Vec<_>>>()?
                            .join("\n---| ")
                    )?;
                }
                Type::Alias(ty) => {
                    if let Some(docs) = self.accumulate_docs(&[definition.doc.as_deref()]) {
                        writeln!(buffer, "{}", docs.join("\n"))?;
                    }
                    writeln!(
                        buffer,
                        "--- @alias {} {}",
                        definition.name,
                        self.type_signature(ty)?
                    )?;
                }
                Type::Function { params, returns } => {
                    if let Some(docs) = self.accumulate_docs(&[definition.doc.as_deref()]) {
                        writeln!(buffer, "{}", docs.join("\n"))?;
                    }
                    writeln!(
                        buffer,
                        "{}",
                        self.function_signature(
                            escape_key(definition.name.as_ref()),
                            params,
                            returns,
                            false
                        )?
                        .join("\n")
                    )?;
                }
                other => {
                    return Err(mlua::Error::runtime(format!(
                        "invalid root level type: {:?}",
                        other
                    )));
                }
            }
            writeln!(buffer)?;
        }

        Ok(())
    }

    fn function_signature<S: std::fmt::Display>(
        &self,
        name: S,
        params: &[Param],
        returns: &[Return],
        assign: bool,
    ) -> mlua::Result<Vec<String>> {
        let mut result = Vec::new();

        for (i, param) in params.iter().enumerate() {
            let ty = self.type_signature(&param.ty)?;
            let doc = param.doc.as_deref().filter(|d| !d.is_empty());
            result.push(match (param.name.as_deref(), doc) {
                (Some(name), Some(doc)) => format!("--- @param {name} {ty} {doc}"),
                (Some(name), None) => format!("--- @param {name} {ty}"),
                (None, Some(doc)) => format!("--- @param param{} {ty} {doc}", i + 1),
                (None, None) => format!("--- @param param{} {ty}", i + 1),
            });
        }

        for (i, ret) in returns.iter().enumerate() {
            let ty = self.type_signature(&ret.ty)?;
            let doc = ret.doc.as_deref().filter(|d| !d.is_empty());
            result.push(match doc {
                Some(doc) => format!("--- @return {ty} #{}: {doc}", i + 1),
                None => format!("--- @return {ty}"),
            });
        }

        result.push(format!(
            "{}function{}({}) end",
            if assign {
                format!("{name} = ")
            } else {
                String::new()
            },
            if !assign {
                format!(" {name}")
            } else {
                String::new()
            },
            params
                .iter()
                .enumerate()
                .map(|(i, v)| v
                    .name
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or(format!("param{}", i + 1)))
                .collect::<Vec<_>>()
                .join(", "),
        ));
        Ok(result)
    }

    fn method_signature<S: std::fmt::Display>(
        &self,
        name: S,
        class: String,
        params: &[Param],
        returns: &[Return],
        assign: bool,
    ) -> mlua::Result<Vec<String>> {
        let mut result = Vec::from([format!("--- @param self {class}")]);
        for (i, param) in params.iter().enumerate() {
            let ty = self.type_signature(&param.ty)?;
            let doc = param.doc.as_deref().filter(|d| !d.is_empty());
            result.push(match (param.name.as_deref(), doc) {
                (Some(name), Some(doc)) => format!("--- @param {name} {ty} {doc}"),
                (Some(name), None) => format!("--- @param {name} {ty}"),
                (None, Some(doc)) => format!("--- @param param{} {ty} {doc}", i + 1),
                (None, None) => format!("--- @param param{} {ty}", i + 1),
            });
        }

        for (i, ret) in returns.iter().enumerate() {
            let ty = self.type_signature(&ret.ty)?;
            let doc = ret.doc.as_deref().filter(|d| !d.is_empty());
            result.push(match doc {
                Some(doc) => format!("--- @return {ty} #{}: {doc}", i + 1),
                None => format!("--- @return {ty}"),
            });
        }

        result.push(format!(
            "{}function{}({}{}) end",
            if assign {
                format!("{name} = ")
            } else {
                String::new()
            },
            if !assign {
                format!(" {name}")
            } else {
                String::new()
            },
            if params.is_empty() { "self" } else { "self, " },
            params
                .iter()
                .enumerate()
                .map(|(i, v)| v
                    .name
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or(format!("param{}", i + 1)))
                .collect::<Vec<_>>()
                .join(", "),
        ));
        Ok(result)
    }

    fn type_signature(&self, ty: &Type) -> mlua::Result<String> {
        Ok(match ty {
            Type::Enum(_) => match self.name_map.borrow().get(ty) {
                Some(name) => name.to_string(),
                None => {
                    return Err(mlua::Error::runtime(
                        "missing enum type definition; make sure the type is registered before it is used",
                    ));
                }
            },
            Type::Class(_) => match self.name_map.borrow().get(ty) {
                Some(name) => name.to_string(),
                None => {
                    return Err(mlua::Error::runtime(
                        "missing class type definition; make sure the type is registered before it is used",
                    ));
                }
            },
            Type::Single(value) => value.to_string(),
            Type::Tuple(types) => {
                format!(
                    "[{}]",
                    types
                        .iter()
                        .map(|v| self.type_signature(v))
                        .collect::<mlua::Result<Vec<_>>>()?
                        .join(", ")
                )
            }
            Type::Array(ty) => {
                format!("{}[]", self.type_signature(ty)?)
            }
            Type::Map(key, value) => {
                format!(
                    "{{ [{}]: {} }}",
                    self.type_signature(key)?,
                    self.type_signature(value)?
                )
            }
            Type::Function { params, returns } => {
                format!(
                    "fun({}){}",
                    params
                        .iter()
                        .enumerate()
                        .map(|(i, v)| {
                            let name = v
                                .name
                                .as_ref()
                                .map(|n| n.to_string())
                                .unwrap_or(format!("param{}", i + 1));
                            Ok(format!("{name}: {}", self.type_signature(&v.ty)?))
                        })
                        .collect::<mlua::Result<Vec<_>>>()?
                        .join(", "),
                    if returns.is_empty() {
                        String::new()
                    } else {
                        format!(
                            ": {}",
                            returns
                                .iter()
                                .map(|v| self.type_signature(&v.ty))
                                .collect::<mlua::Result<Vec<_>>>()?
                                .join(", ")
                        )
                    }
                )
            }
            Type::Union(types) => types
                .iter()
                .map(|v| self.type_signature(v))
                .collect::<mlua::Result<Vec<_>>>()?
                .join(" | "),
            Type::Table(entries) => {
                format!(
                    "{{ {} }}",
                    entries
                        .iter()
                        .map(|(k, v)| { Ok(format!("{k}: {}", self.type_signature(v)?)) })
                        .collect::<mlua::Result<Vec<_>>>()?
                        .join(", ")
                )
            }
            other => {
                return Err(mlua::Error::runtime(format!(
                    "type cannot be a type signature: {}",
                    other.as_ref()
                )));
            }
        })
    }

    fn accumulate_docs(&self, docs: &[Option<&str>]) -> Option<Vec<String>> {
        let docs = docs.iter().filter_map(|v| *v).collect::<Vec<_>>();
        (!docs.is_empty()).then_some({
            docs.iter()
                .flat_map(|v| v.split('\n').map(|v| format!("--- {v}")))
                .collect::<Vec<_>>()
        })
    }
}

fn needs_escape(key: &str) -> bool {
    key.chars().any(|v| !v.is_alphanumeric() && v != '_')
}

fn escape_key(key: &str) -> String {
    if needs_escape(key) {
        format!(r#"["{key}"]"#)
    } else {
        key.to_string()
    }
}
