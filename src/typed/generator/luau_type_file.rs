use std::{cell::RefCell, collections::HashMap, path::Path, slice::Iter};

use crate::typed::{Param, Type, function::Return};

use super::{Definition, Definitions};

/// Generates Luau definition files (`.d.luau`) for each
/// [`Definition`][`crate::typed::generator::Definition`].
///
/// Each file uses native Luau type syntax: `declare class`, `declare function`,
/// `type` aliases, and `declare` for global values. These files are consumed by
/// the Luau type checker and by luau-lsp.
///
/// # Example Output
///
/// ```luau
/// -- Example class
/// declare class Example
///     name: string
///     function run(self): boolean
/// end
///
/// declare example: Example
/// ```
pub struct LuauDefinitionFileGenerator {
    extension: String,
    definitions: Definitions,
}

impl Default for LuauDefinitionFileGenerator {
    fn default() -> Self {
        Self {
            extension: ".d.luau".into(),
            definitions: Definitions::default(),
        }
    }
}

impl LuauDefinitionFileGenerator {
    pub fn new(definitions: Definitions) -> Self {
        Self {
            definitions,
            ..Default::default()
        }
    }

    pub fn ext(mut self, ext: impl AsRef<str>) -> Self {
        self.extension = ext.as_ref().to_string();
        self
    }

    pub fn iter(&self) -> LuauDefinitionFileIter<'_> {
        LuauDefinitionFileIter {
            extension: self.extension.clone(),
            definitions: self.definitions.iter(),
        }
    }
}

pub struct LuauDefinitionFileIter<'def> {
    extension: String,
    definitions: Iter<'def, (String, Definition)>,
}

impl<'def> Iterator for LuauDefinitionFileIter<'def> {
    type Item = (String, LuauDefinitionWriter<'def>);

    fn next(&mut self) -> Option<Self::Item> {
        self.definitions.next().map(|v| {
            (
                format!("{}{}", v.0, self.extension),
                LuauDefinitionWriter {
                    definition: &v.1,
                    name_map: RefCell::new(HashMap::default()),
                },
            )
        })
    }
}

pub struct LuauDefinitionWriter<'def> {
    definition: &'def Definition,
    name_map: RefCell<HashMap<Type, String>>,
}

impl<'writer> LuauDefinitionWriter<'writer> {
    pub fn write_file<P: AsRef<Path>>(self, path: P) -> mlua::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)?;
        self.write(&mut file)
    }

    pub fn write<W: std::io::Write>(self, mut buffer: W) -> mlua::Result<()> {
        let mut first = true;

        for definition in self.definition.iter() {
            if !first {
                writeln!(buffer)?;
            }
            first = false;

            match &definition.ty {
                Type::Value(ty) => {
                    self.write_doc_comments(&mut buffer, &[definition.doc.as_deref()], "")?;
                    writeln!(
                        buffer,
                        "declare {}: {}",
                        definition.name,
                        self.type_signature(ty)?
                    )?;
                }
                Type::Class(type_data) => {
                    self.name_map
                        .borrow_mut()
                        .insert(definition.ty.clone(), definition.name.clone());

                    self.write_doc_comments(
                        &mut buffer,
                        &[definition.doc.as_deref(), type_data.type_doc.as_deref()],
                        "",
                    )?;
                    write!(buffer, "declare class {}", definition.name)?;
                    if !type_data.derives.is_empty() {
                        write!(buffer, " extends {}", type_data.derives.join(", "))?;
                    }
                    writeln!(buffer)?;

                    // Instance fields
                    for (name, field) in type_data.fields.iter() {
                        if name.is_int() {
                            continue;
                        }
                        self.write_doc_comments(&mut buffer, &[field.doc.as_deref()], "\t")?;
                        writeln!(buffer, "\t{}: {}", name, self.type_signature(&field.ty)?)?;
                    }

                    // Methods (with self)
                    for (name, func) in type_data.methods.iter() {
                        self.write_doc_comments(&mut buffer, &[func.doc.as_deref()], "\t")?;
                        writeln!(
                            buffer,
                            "\tfunction {}(self{}{}): {}",
                            name,
                            if func.params.is_empty() { "" } else { ", " },
                            self.param_list(&func.params)?,
                            self.return_type(&func.returns)?,
                        )?;
                    }

                    // Meta fields
                    for (name, field) in type_data.meta_fields.iter() {
                        self.write_doc_comments(&mut buffer, &[field.doc.as_deref()], "\t")?;
                        writeln!(buffer, "\t{}: {}", name, self.type_signature(&field.ty)?)?;
                    }

                    // Meta methods (with self)
                    for (name, func) in type_data.meta_methods.iter() {
                        self.write_doc_comments(&mut buffer, &[func.doc.as_deref()], "\t")?;
                        self.write_param_doc_comments(&mut buffer, &func.params, "\t")?;
                        self.write_return_doc_comments(&mut buffer, &func.returns, "\t")?;
                        writeln!(
                            buffer,
                            "\tfunction {}(self{}{}): {}",
                            name,
                            if func.params.is_empty() { "" } else { ", " },
                            self.param_list(&func.params)?,
                            self.return_type(&func.returns)?,
                        )?;
                    }

                    writeln!(buffer, "end")?;

                    // Static functions and meta_functions are emitted as a
                    // separate global table declaration, since `declare class`
                    // requires `self` on every function.
                    //
                    // They are first declared themselves to give them richer type information.
                    // Then they are added to a global table declaration with `typeof()`.
                    let static_fns: Vec<_> = type_data
                        .functions
                        .iter()
                        .chain(type_data.meta_functions.iter())
                        .collect();

                    if !static_fns.is_empty() {
                        writeln!(buffer)?;
                    }

                    for (name, func) in static_fns.iter() {
                        self.write_doc_comments(&mut buffer, &[func.doc.as_deref()], "")?;
                        self.write_param_doc_comments(&mut buffer, &func.params, "")?;
                        self.write_return_doc_comments(&mut buffer, &func.returns, "")?;
                        writeln!(
                            buffer,
                            "declare function {}_{name}({}): {}",
                            definition.name,
                            self.param_list(&func.params)?,
                            self.return_type(&func.returns)?,
                        )?;
                    }

                    if !static_fns.is_empty() || !type_data.static_fields.is_empty() {
                        writeln!(buffer)?;
                        writeln!(buffer, "declare {}: {{", definition.name)?;

                        for (name, field) in type_data.static_fields.iter() {
                            self.write_doc_comments(
                                &mut buffer,
                                &[field.inner.doc.as_deref()],
                                "\t",
                            )?;
                            writeln!(
                                buffer,
                                "\t{}: {},",
                                name,
                                self.type_signature(&field.inner.ty)?
                            )?;
                        }

                        for (name, _func) in &static_fns {
                            writeln!(buffer, "\t{name}: typeof({}_{name}),", definition.name,)?;
                        }
                        writeln!(buffer, "}}")?;
                    }
                }
                Type::Enum(types) => {
                    self.name_map
                        .borrow_mut()
                        .insert(definition.ty.clone(), definition.name.clone());

                    self.write_doc_comments(&mut buffer, &[definition.doc.as_deref()], "")?;
                    let type_strs = types
                        .iter()
                        .map(|v| self.type_signature(v))
                        .collect::<mlua::Result<Vec<_>>>()?;
                    writeln!(
                        buffer,
                        "export type {} = {}",
                        definition.name,
                        type_strs.join(" | "),
                    )?;
                }
                Type::Alias(ty) => {
                    self.write_doc_comments(&mut buffer, &[definition.doc.as_deref()], "")?;
                    writeln!(
                        buffer,
                        "export type {} = {}",
                        definition.name,
                        self.type_signature(ty)?,
                    )?;
                }
                Type::Function { params, returns } => {
                    self.write_doc_comments(&mut buffer, &[definition.doc.as_deref()], "")?;
                    self.write_param_doc_comments(&mut buffer, &params, "")?;
                    self.write_return_doc_comments(&mut buffer, &returns, "")?;
                    writeln!(
                        buffer,
                        "declare function {}({}): {}",
                        definition.name,
                        self.param_list(params)?,
                        self.return_type(returns)?,
                    )?;
                }
                other => {
                    return Err(mlua::Error::runtime(format!(
                        "invalid root level type: {:?}",
                        other
                    )));
                }
            }
        }

        Ok(())
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
            Type::Single(value) => {
                match value.as_ref() {
                    // Luau has `userdata` but it isn't an actual type but instead
                    // defined luau class types. This will erase the generic userdata
                    // types to `any`.
                    "userdata" | "lightuserdata" => "any".into(),
                    // Luau recognizes `integer` as a type, but numeric literals
                    // are inferred as `number` and the two are mutually incompatible,
                    // making `integer` unusable in practice. Emit `number` instead.
                    "integer" => "number".to_string(),
                    other => other.to_string(),
                }
            }
            Type::Tuple(types) => {
                // Luau doesn't support integer literal keys in table types.
                // If all element types are the same, emit as {T}.
                // Otherwise emit as a union of the element types, which is
                // the closest approximation without generics/type packs.
                if types.is_empty() {
                    "{}".to_string()
                } else if types.iter().all(|t| t == &types[0]) {
                    format!("{{ {} }}", self.type_signature(&types[0])?)
                } else {
                    // Collect unique types and emit as {T1 | T2 | ...}
                    let mut seen = Vec::new();
                    for t in types {
                        if !seen.contains(t) {
                            seen.push(t.clone());
                        }
                    }
                    let sigs = seen
                        .iter()
                        .map(|v| self.type_signature(v))
                        .collect::<mlua::Result<Vec<_>>>()?;
                    format!("{{ {} }}", sigs.join(" | "))
                }
            }
            Type::Array(ty) => {
                format!("{{ {} }}", self.type_signature(ty)?)
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
                    "({}) -> {}",
                    self.param_list(params)?,
                    self.return_type(returns)?,
                )
            }
            Type::Union(types) => {
                // Check for T | nil pattern and emit T? shorthand
                if types.len() == 2 {
                    let nil_pos = types
                        .iter()
                        .position(|t| matches!(t, Type::Single(s) if s == "nil"));
                    if let Some(pos) = nil_pos {
                        let other = &types[1 - pos];
                        let sig = self.type_signature(other)?;
                        // Wrap complex types in parens before adding ?
                        return Ok(if needs_parens_for_optional(other) {
                            format!("({sig})?")
                        } else {
                            format!("{sig}?")
                        });
                    }
                }
                types
                    .iter()
                    .map(|v| self.type_signature(v))
                    .collect::<mlua::Result<Vec<_>>>()?
                    .join(" | ")
            }
            Type::Table(entries) => {
                let fields = entries
                    .iter()
                    .map(|(k, v)| Ok(format!("{k}: {}", self.type_signature(v)?)))
                    .collect::<mlua::Result<Vec<_>>>()?;
                format!("{{ {} }}", fields.join(", "))
            }
            other => {
                return Err(mlua::Error::runtime(format!(
                    "type cannot be a type signature: {}",
                    other.as_ref()
                )));
            }
        })
    }

    fn param_list(&self, params: &[Param]) -> mlua::Result<String> {
        params
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let name = p
                    .name
                    .as_deref()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| format!("param{}", i + 1));
                Ok(format!("{}: {}", name, self.type_signature(&p.ty)?))
            })
            .collect::<mlua::Result<Vec<_>>>()
            .map(|v| v.join(", "))
    }

    fn return_type(&self, returns: &[Return]) -> mlua::Result<String> {
        if returns.is_empty() {
            return Ok("()".to_string());
        }
        let sigs = returns
            .iter()
            .map(|r| self.type_signature(&r.ty))
            .collect::<mlua::Result<Vec<_>>>()?;
        if sigs.len() == 1 {
            Ok(sigs.into_iter().next().unwrap())
        } else {
            Ok(format!("({})", sigs.join(", ")))
        }
    }

    fn write_doc_comments<W: std::io::Write>(
        &self,
        buffer: &mut W,
        docs: &[Option<&str>],
        indent: &str,
    ) -> mlua::Result<()> {
        for doc in docs.iter().filter_map(|v| *v) {
            for line in doc.split('\n') {
                writeln!(buffer, "{indent}-- {line}")?;
            }
        }
        Ok(())
    }

    fn write_param_doc_comments<W: std::io::Write>(
        &self,
        buffer: &mut W,
        params: &[Param],
        indent: &str,
    ) -> mlua::Result<()> {
        for (i, p) in params.iter().enumerate().filter(|(_, p)| p.doc.is_some()) {
            write!(
                buffer,
                "{indent}-- @param {} {}",
                p.name
                    .as_deref()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| format!("param{}", i + 1)),
                self.type_signature(&p.ty)?
            )?;
            if let Some(doc) = p.doc.as_deref() {
                let doc = doc.replace('\n', "");
                write!(buffer, " -- {doc}")?;
            }
            writeln!(buffer)?;
        }
        Ok(())
    }

    fn write_return_doc_comments<W: std::io::Write>(
        &self,
        buffer: &mut W,
        returns: &[Return],
        indent: &str,
    ) -> mlua::Result<()> {
        for (i, r) in returns.iter().enumerate().filter(|(_, r)| r.doc.is_some()) {
            write!(buffer, "{indent}-- @return {}", self.type_signature(&r.ty)?)?;
            if let Some(doc) = r.doc.as_deref() {
                let doc = doc.replace('\n', "");
                write!(buffer, " -- #{} {doc}", i + 1)?;
            }
            writeln!(buffer)?;
        }
        Ok(())
    }
}

fn needs_parens_for_optional(ty: &Type) -> bool {
    matches!(ty, Type::Union(_) | Type::Function { .. })
}
