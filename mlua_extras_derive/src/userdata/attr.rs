use proc_macro2::Span;
use syn::ext::IdentExt;
use syn::meta::ParseNestedMeta;
use syn::spanned::Spanned;
use syn::{Attribute, Ident, LitStr, Meta, Result};

/// Parsed `#[lua(...)]` attribute.
///
/// Some fields are based on context:
/// - `get`, `set`, `name`, and `skip` are for structs
/// - `getter`, `setter`, `field`, `meta`, `infallible`, `name`, and `skip` are used for impl methods
#[derive(Default)]
pub(crate) struct Attr {
    pub span: Option<Span>,
    pub name: Option<String>,
    pub infallible: bool,
    pub skip: bool,

    pub get: bool,
    pub set: bool,

    pub getter: bool,
    pub setter: bool,
    pub field: bool,
    pub meta: bool,
}

impl Attr {
    pub fn parse(&mut self, meta: ParseNestedMeta) -> Result<()> {
        match &meta.path {
            path if path.is_ident("skip") => {
                if meta.value().is_ok() {
                    return Err(meta.error("`skip` does not take a value"));
                }
                self.skip = true;
            },
            path if path.is_ident("infallible") => {
                if meta.value().is_ok() {
                    return Err(meta.error("`infallible` does not take a value"));
                }
                self.infallible = true;
            },
            path if path.is_ident("get") => self.get = true,
            path if path.is_ident("set") => self.set = true,
            path if path.is_ident("getter") => self.getter = true,
            path if path.is_ident("setter") => self.setter = true,
            path if path.is_ident("field") => self.field = true,
            path if path.is_ident("meta") => self.meta = true,
            path if path.is_ident("name") => {
                let value = meta.value()?;
                let lit: LitStr = value.parse()?;
                self.name = Some(lit.value());
            },
            _ => {
                return Err(meta.error(
                    "unsupported lua attribute, expected: ".to_string()
                        + "`skip`, `infallible`, `get`, `set`, `getter`, `setter`, `field`, `meta`, `name`"
                ))
            }
        }
        Ok(())
    }

    /// Returns the name or the default ident representation.
    pub fn name_or_default(&self, ident: &Ident) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| ident.unraw().to_string())
    }

    pub fn span(&self) -> Span {
        self.span.unwrap_or_else(Span::call_site)
    }

    pub fn meta_name(&self, fn_ident: &Ident) -> Result<String> {
        if let Some(ref name) = self.name {
            return Ok(name.clone());
        }

        let fn_name = fn_ident.unraw().to_string();
        if fn_name.starts_with("__") {
            return Ok(fn_name);
        }

        Err(syn::Error::new(
            fn_ident.span(),
            format!(
                "could not infer metamethod name from `{fn_name}`, either add `name = \"...\"` to `#[lua(meta, ...)]` oro prefix the function with `__`"
            )
        ))
    }
}

pub(crate) fn parse_attrs<V: Fn(&Attr) -> Result<()>>(
    attrs: &[Attribute],
    validate: V,
) -> Result<Attr> {
    let mut lua_attr = Attr::default();

    for attr in attrs {
        if !attr.path().is_ident("lua") {
            continue;
        }

        match &attr.meta {
            Meta::List(_) => {
                _ = lua_attr.span.replace(attr.span());
                attr.parse_nested_meta(|meta| lua_attr.parse(meta))?;
                validate(&lua_attr)?;
            }
            Meta::Path(_) => {}
            Meta::NameValue(_) => {
                return Err(syn::Error::new_spanned(
                    attr,
                    "`#[lua = \"...\"]` is not supported: use `#[lua(attr = \"...\")]`",
                ));
            }
        }
    }

    Ok(lua_attr)
}

pub(crate) fn validate_field_attr(attr: &Attr) -> Result<()> {
    for (set, name) in [
        (attr.getter, "getter"),
        (attr.setter, "setter"),
        (attr.field, "field"),
        (attr.meta, "meta"),
        (attr.infallible, "infallible"),
    ] {
        if set {
            return Err(syn::Error::new(
                attr.span(),
                format!("`{name}` is unsupported for struct fields"),
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_method_attr(attr: &Attr) -> Result<()> {
    for (set, name) in [(attr.get, "get"), (attr.set, "set")] {
        if set {
            return Err(syn::Error::new(
                attr.span(),
                format!("`{name}` is unsupported for methods"),
            ));
        }
    }
    Ok(())
}
