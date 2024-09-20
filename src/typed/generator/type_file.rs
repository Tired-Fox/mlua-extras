use std::{path::Path, slice::Iter};

use crate::typed::{Param, Type};

use super::{DefinitionGroup, Definitions};

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
pub struct TypeFileGenerator<'def> {
    /// Extendion of each definition file: Default [`.d.lua`]
    ///
    /// **IMPORTANT** Must start with a dot
    extension: String,
    definitions: Definitions<'def>,
}

impl<'def> Default for TypeFileGenerator<'def> {
    fn default() -> Self {
        Self {
            extension: ".d.lua".into(),
            definitions: Definitions::default(),
        }
    }
}

impl<'def> TypeFileGenerator<'def> {
    /// Create a new generator given a collection of definitions
    pub fn new(definitions: Definitions<'def>) -> Self {
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

    pub fn iter(&self) -> TypeFileIter<'_> {
        TypeFileIter {
            extension: self.extension.clone(),
            definitions: self.definitions.iter(),
        }
    }
}

pub struct TypeFileIter<'def> {
    extension: String,
    definitions: Iter<'def, DefinitionGroup<'def>>,
}

impl<'def> Iterator for TypeFileIter<'def> {
    type Item = (String, DefinitionWriter<'def>);

    fn next(&mut self) -> Option<Self::Item> {
        self.definitions.next().map(|v| {
            (
                format!("{}{}", v.name, self.extension),
                DefinitionWriter { definition: v },
            )
        })
    }
}

pub struct DefinitionWriter<'def> {
    definition: &'def DefinitionGroup<'def>,
}

impl DefinitionWriter<'_> {
    /// Write the full definition group to a specified file
    pub fn write_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)?;
        self.write(&mut file)
    }

    /// PERF: Check if there is a good api for adding color when printing to stdout, stderr, etc
    ///
    /// Write the full definition group to the specified `io`
    pub fn write<W: std::io::Write>(&self, mut buffer: W) -> std::io::Result<()> {
        writeln!(buffer, "--- @meta\n")?;

        // TODO: Iterate the definitions and output the lua code
        for definition in self.definition.iter() {
            match &definition.ty {
                Type::Value(ty) => {
                    if let Some(docs) = Self::accumulate_docs(&[&definition.docs]) {
                        writeln!(buffer, "{}", docs.join("\n"))?;
                    }

                    writeln!(buffer, "--- @type {}", Self::type_signature(ty))?;
                    writeln!(buffer, "{} = nil", definition.name)?;
                    writeln!(buffer)?;
                }
                Type::Class(type_data) => {
                    if let Some(docs) =
                        Self::accumulate_docs(&[&definition.docs, &type_data.type_doc])
                    {
                        writeln!(buffer, "{}", docs.join("\n"))?;
                    }
                    writeln!(buffer, "--- @class {}", definition.name)?;

                    // TODO: meta_fields
                    // TODO: meta_methods
                    // TODO: meta_functions

                    for (name, field) in type_data.static_fields.iter() {
                        if let Some(docs) = Self::accumulate_docs(&[&field.docs]) {
                            writeln!(buffer, "{}", docs.join("\n"))?;
                        }
                        writeln!(
                            buffer,
                            "--- @field {name} {}",
                            Self::type_signature(&field.ty)
                        )?;
                    }

                    for (name, field) in type_data.fields.iter() {
                        if let Some(docs) = Self::accumulate_docs(&[&field.docs]) {
                            writeln!(buffer, "{}", docs.join("\n"))?;
                        }
                        writeln!(
                            buffer,
                            "--- @field {name} {}",
                            Self::type_signature(&field.ty)
                        )?;
                    }

                    if !type_data.functions.is_empty()
                        || !type_data.methods.is_empty()
                        || !type_data.meta_fields.is_empty()
                        || !type_data.meta_fields.is_empty()
                        || !type_data.meta_functions.is_empty()
                        || !type_data.meta_methods.is_empty()
                    {
                        writeln!(buffer, "local _Class_{} = {{", definition.name)?;
                        for (name, func) in type_data.functions.iter() {
                            if let Some(docs) = Self::accumulate_docs(&[&func.docs]) {
                                writeln!(buffer, "  {}", docs.join("\n  "))?;
                            }
                            writeln!(buffer, "  {},", Self::function_signature(name.to_string(), &func.params, &func.returns, true).join("\n  "))?;
                        }

                        for (name, func) in type_data.methods.iter() {
                            if let Some(docs) = Self::accumulate_docs(&[&func.docs]) {
                                writeln!(buffer, "  {}", docs.join("\n  "))?;
                            }
                            writeln!(buffer, "  {},", Self::method_signature(name.to_string(), definition.name.to_string(), &func.params, &func.returns, true).join("\n  "))?;
                        }
                        
                        if !type_data.meta_fields.is_empty()
                            || !type_data.meta_functions.is_empty()
                            || !type_data.meta_methods.is_empty() {

                            writeln!(buffer, "  __metatable = {{")?;
                            for (name, field) in type_data.meta_fields.iter() {
                                if let Some(docs) = Self::accumulate_docs(&[&field.docs]) {
                                    writeln!(buffer, "    {}", docs.join("\n    "))?;
                                }
                                writeln!(buffer, "--- @type {}", Self::type_signature(&field.ty))?;
                                writeln!(buffer, "{name} = nil,")?;
                            }

                            for (name, func) in type_data.meta_functions.iter() {
                                if let Some(docs) = Self::accumulate_docs(&[&func.docs]) {
                                    writeln!(buffer, "    {}", docs.join("\n    "))?;
                                }
                                writeln!(buffer, "    {},", Self::function_signature(name.to_string(), &func.params, &func.returns, true).join("\n    "))?;
                            }

                            for (name, func) in type_data.meta_methods.iter() {
                                if let Some(docs) = Self::accumulate_docs(&[&func.docs]) {
                                    writeln!(buffer, "    {}", docs.join("\n    "))?;
                                }
                                writeln!(buffer, "    {},", Self::method_signature(name.to_string(), definition.name.to_string(), &func.params, &func.returns, true).join("\n    "))?;
                            }
                            writeln!(buffer, "  }}")?;
                        }

                        writeln!(buffer, "}}")?;
                    }

                    writeln!(buffer)?;
                }
                Type::Enum(name, types) => {
                    if let Some(docs) = Self::accumulate_docs(&[&definition.docs]) {
                        writeln!(buffer, "{}", docs.join("\n"))?;
                    }
                    writeln!(
                        buffer,
                        "--- @alias {name} {}",
                        types
                            .iter()
                            .map(Self::type_signature)
                            .collect::<Vec<_>>()
                            .join("\n---  | ")
                    )?;
                    writeln!(buffer)?;
                },
                Type::Alias(ty) => {
                    if let Some(docs) = Self::accumulate_docs(&[&definition.docs]) {
                        writeln!(buffer, "{}", docs.join("\n"))?;
                    }
                    writeln!(
                        buffer,
                        "--- @alias {} {}",
                        definition.name,
                        Self::type_signature(ty)
                    )?;
                    writeln!(buffer)?;
                }
                Type::Function { params, returns } => {
                    if let Some(docs) = Self::accumulate_docs(&[&definition.docs]) {
                        writeln!(buffer, "{}", docs.join("\n"))?;
                    }
                    writeln!(buffer, "{}", Self::function_signature(definition.name.to_string(), params, returns, false).join("\n"))?;
                }
                other => unimplemented!("Definition format for `{}`: `{}`", definition.name, other.as_ref()),
            }
        }

        Ok(())
    }

    fn function_signature(name: String, params: &[Param], returns: &[Type], assign: bool) -> Vec<String> {
        let mut result = Vec::new();

        result.extend(params
            .iter()
            .enumerate()
            .map(|(i, v)| format!("--- @param {} {}", v.name.as_ref().map(|v| v.to_string()).unwrap_or(format!("param{i}")), Self::type_signature(&v.ty)))
            .chain(returns.iter().map(|v| format!("--- @return {}", Self::type_signature(v))))
        );



        result.push(format!(
            "{}function{}({}) end",
            if assign {
                format!("{name} = ")
            } else { String::new() },
            if !assign {
                format!(" {name}")
            } else { String::new() },
            params
                .iter()
                .enumerate()
                .map(|(i, v)| v.name
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or(format!("param{i}")))
                .collect::<Vec<_>>()
                .join(", "),
        ));
        result
    }

    fn method_signature(name: String, class: String, params: &[Param], returns: &[Type], assign: bool) -> Vec<String> {
        let mut result = Vec::from([format!("--- @param self {class}")]);
        result.extend(params
            .iter()
            .enumerate()
            .map(|(i, v)| format!("--- @param {} {}", v.name.as_ref().map(|v| v.to_string()).unwrap_or(format!("param{i}")), Self::type_signature(&v.ty)))
            .chain(returns.iter().map(|v| format!("--- @return {}", Self::type_signature(v))))
        );



        result.push(format!(
            "{}function{}({}{}) end",
            if assign {
                format!("{name} = ")
            } else { String::new() },
            if !assign {
                format!(" {name}")
            } else { String::new() },
            if params.is_empty() {
                "self"
            } else {
                "self, "
            },
            params
                .iter()
                .enumerate()
                .map(|(i, v)| v.name
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or(format!("param{i}")))
                .collect::<Vec<_>>()
                .join(", "),
        ));
        result
    }

    fn type_signature(ty: &Type) -> String {
        match ty {
            Type::Enum(name, _) => name.to_string(),
            Type::Single(value) => value.to_string(),
            Type::Tuple(types) => {
                format!(
                    "{{ {} }}",
                    types
                        .iter()
                        .enumerate()
                        .map(|(i, t)| { format!("[{}]: {}", i + 1, Self::type_signature(t)) })
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            other => unimplemented!("Type signature for `{}`", other.as_ref()),
        }
    }

    fn accumulate_docs(docs: &[&[String]]) -> Option<Vec<String>> {
        let docs = docs.iter().flat_map(|v| *v).collect::<Vec<_>>();
        (!docs.is_empty())
            .then_some({
                docs
                    .iter()
                    .flat_map(|v| v.split('\n').map(|v| format!("--- {v}")))
                    .collect::<Vec<_>>()
            })
    }
}
