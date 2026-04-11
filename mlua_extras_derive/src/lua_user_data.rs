use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Lit, Meta};

#[derive(Default)]
struct FieldAttr {
    skip: bool,
    readonly: bool,
    writeonly: bool,
    rename: Option<String>,
}

enum FieldAccess {
    ReadWrite,
    ReadOnly,
    WriteOnly,
    Skip,
}

struct FieldInfo {
    ident: syn::Ident,
    ty: syn::Type,
    lua_name: String,
    access: FieldAccess,
    doc: Option<String>,
}

fn parse_field_attr(field: &syn::Field) -> FieldAttr {
    let mut attr = FieldAttr::default();

    for a in &field.attrs {
        if !a.path().is_ident("mlua_extras") {
            continue;
        }
        match &a.meta {
            Meta::List(list) => {
                // Parse the token stream manually to support bare flags and key=value
                let parsed: syn::punctuated::Punctuated<syn::Meta, syn::Token![,]> =
                    match list.parse_args_with(syn::punctuated::Punctuated::parse_terminated) {
                        Ok(v) => v,
                        Err(e) => {
                            proc_macro_error::abort!(list.tokens, "invalid mlua_extras attribute: {}", e);
                        }
                    };

                for meta in parsed {
                    match &meta {
                        Meta::Path(path) => {
                            if path.is_ident("skip") {
                                attr.skip = true;
                            } else if path.is_ident("readonly") {
                                attr.readonly = true;
                            } else if path.is_ident("writeonly") {
                                attr.writeonly = true;
                            } else {
                                proc_macro_error::abort!(path, "unknown attribute");
                            }
                        }
                        Meta::NameValue(nv) => {
                            if nv.path.is_ident("rename") {
                                if let syn::Expr::Lit(expr_lit) = &nv.value {
                                    if let Lit::Str(s) = &expr_lit.lit {
                                        attr.rename = Some(s.value());
                                    } else {
                                        proc_macro_error::abort!(expr_lit, "expected string literal for rename");
                                    }
                                } else {
                                    proc_macro_error::abort!(nv.value, "expected string literal for rename");
                                }
                            } else {
                                proc_macro_error::abort!(nv.path, "unknown attribute");
                            }
                        }
                        _ => {
                            proc_macro_error::abort!(meta, "unexpected attribute form");
                        }
                    }
                }
            }
            _ => {
                proc_macro_error::abort!(a, "expected #[mlua_extras(...)]");
            }
        }
    }

    attr
}

fn extract_doc_comment(attrs: &[syn::Attribute]) -> Option<String> {
    let docs: Vec<String> = attrs
        .iter()
        .filter_map(|attr| {
            if !attr.path().is_ident("doc") {
                return None;
            }
            if let Meta::NameValue(nv) = &attr.meta {
                if let syn::Expr::Lit(expr_lit) = &nv.value {
                    if let Lit::Str(s) = &expr_lit.lit {
                        return Some(s.value().trim().to_string());
                    }
                }
            }
            None
        })
        .collect();

    if docs.is_empty() {
        None
    } else {
        Some(docs.join("\n"))
    }
}

pub fn derive(input: DeriveInput) -> TokenStream {
    let name = &input.ident;
    let struct_doc_stmt = extract_doc_comment(&input.attrs).map(|doc| {
        quote! { docs.add(#doc); }
    });

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => &named.named,
            Fields::Unnamed(_) => {
                proc_macro_error::abort!(name, "TypedUserData does not support tuple structs");
            }
            Fields::Unit => {
                return generate_unit(name, struct_doc_stmt);
            }
        },
        Data::Enum(_) => {
            proc_macro_error::abort!(name, "TypedUserData does not support enums");
        }
        Data::Union(_) => {
            proc_macro_error::abort!(name, "TypedUserData does not support unions");
        }
    };

    let field_infos: Vec<FieldInfo> = fields
        .iter()
        .filter_map(|field| {
            let ident = field.ident.as_ref()?;
            let attr = parse_field_attr(field);
            let access = if attr.skip {
                FieldAccess::Skip
            } else if attr.readonly {
                FieldAccess::ReadOnly
            } else if attr.writeonly {
                FieldAccess::WriteOnly
            } else {
                FieldAccess::ReadWrite
            };
            let lua_name = attr.rename.unwrap_or_else(|| ident.to_string());
            let doc = extract_doc_comment(&field.attrs);

            Some(FieldInfo {
                ident: ident.clone(),
                ty: field.ty.clone(),
                lua_name,
                access,
                doc,
            })
        })
        .collect();

    let field_registrations = field_infos.iter().filter_map(|fi| {
        let lua_name = &fi.lua_name;
        let field_ident = &fi.ident;
        let field_ty = &fi.ty;

        let doc_stmt = fi.doc.as_ref().map(|doc| {
            quote! { fields.document(#doc); }
        });

        match fi.access {
            FieldAccess::Skip => None,
            FieldAccess::ReadWrite => Some(quote! {
                #doc_stmt
                fields.add_field_method_get_set(
                    #lua_name,
                    |_lua, this| Ok(this.#field_ident.clone()),
                    |_lua, this, val: #field_ty| { this.#field_ident = val; Ok(()) },
                );
            }),
            FieldAccess::ReadOnly => Some(quote! {
                #doc_stmt
                fields.add_field_method_get(
                    #lua_name,
                    |_lua, this| Ok(this.#field_ident.clone()),
                );
            }),
            FieldAccess::WriteOnly => Some(quote! {
                #doc_stmt
                fields.add_field_method_set(
                    #lua_name,
                    |_lua, this, val: #field_ty| { this.#field_ident = val; Ok(()) },
                );
            }),
        }
    });

    quote! {
        impl #name {
            #[doc(hidden)]
            fn __auto_add_fields<F: mlua_extras::typed::TypedDataFields<Self>>(fields: &mut F) {
                #(#field_registrations)*
            }
        }

        impl mlua_extras::typed::TypedUserData for #name {
            fn add_documentation<F: mlua_extras::typed::TypedDataDocumentation<Self>>(docs: &mut F) {
                #struct_doc_stmt
            }
            fn add_fields<F: mlua_extras::typed::TypedDataFields<Self>>(fields: &mut F) {
                Self::__auto_add_fields(fields);
            }
            fn add_methods<M: mlua_extras::typed::TypedDataMethods<Self>>(methods: &mut M) {
                #[allow(unused_imports)]
                use mlua_extras::__DefaultAutoMethods;
                Self::__auto_add_methods(methods);
            }
        }

        impl mlua_extras::mlua::UserData for #name {
            fn add_fields<F: mlua_extras::mlua::UserDataFields<Self>>(fields: &mut F) {
                let mut wrapper = mlua_extras::typed::WrappedBuilder::new(fields);
                <#name as mlua_extras::typed::TypedUserData>::add_fields(&mut wrapper);
            }

            fn add_methods<M: mlua_extras::mlua::UserDataMethods<Self>>(methods: &mut M) {
                let mut wrapper = mlua_extras::typed::WrappedBuilder::new(methods);
                <#name as mlua_extras::typed::TypedUserData>::add_methods(&mut wrapper);
            }
        }
    }
}

fn generate_unit(name: &syn::Ident, struct_doc_stmt: Option<TokenStream>) -> TokenStream {
    quote! {
        impl mlua_extras::typed::TypedUserData for #name {
            fn add_documentation<F: mlua_extras::typed::TypedDataDocumentation<Self>>(docs: &mut F) {
                #struct_doc_stmt
            }
            fn add_fields<F: mlua_extras::typed::TypedDataFields<Self>>(fields: &mut F) {
                #[allow(unused_imports)]
                use mlua_extras::__DefaultAutoFields;
                Self::__auto_add_fields(fields);
            }
            fn add_methods<M: mlua_extras::typed::TypedDataMethods<Self>>(methods: &mut M) {
                #[allow(unused_imports)]
                use mlua_extras::__DefaultAutoMethods;
                Self::__auto_add_methods(methods);
            }
        }

        impl mlua_extras::mlua::UserData for #name {
            fn add_fields<F: mlua_extras::mlua::UserDataFields<Self>>(fields: &mut F) {
                let mut wrapper = mlua_extras::typed::WrappedBuilder::new(fields);
                <#name as mlua_extras::typed::TypedUserData>::add_fields(&mut wrapper);
            }

            fn add_methods<M: mlua_extras::mlua::UserDataMethods<Self>>(methods: &mut M) {
                let mut wrapper = mlua_extras::typed::WrappedBuilder::new(methods);
                <#name as mlua_extras::typed::TypedUserData>::add_methods(&mut wrapper);
            }
        }
    }
}
