//! Both mlua and rlua are distributed under the MIT license, which is reproduced
//! below:
//! 
//! MIT License
//! 
//! Copyright (c) 2019-2021 A. Orlenko
//! Copyright (c) 2017 rlua
//! 
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//! 
//! The above copyright notice and this permission notice shall be included in all
//! copies or substantial portions of the Software.
//! 
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//! SOFTWARE.
//!
//! The above notice applies to this file's code as a large portion is copied from, or closely replicates `mlua` to provide
//! a similar experience while giving additional type information to `mlua_extras` typing system.
//! 
//! https://github.com/mlua-rs/mlua/blob/main/mlua_derive/src/userdata/mod.rshttps://github.com/mlua-rs

use proc_macro2::TokenStream;
use syn::{Attribute, Data, DeriveInput, Error, Fields, FieldsNamed, Meta};

pub mod attr;
use attr::validate_field_attr;

use crate::userdata::attr::parse_attrs;

pub mod typed;
pub mod userdata_impl;

/// Wrap registration tokens with any `#[cfg]`/`#[cfg_attr]` attributes from the origional source (span)
pub(crate) fn with_cfg(
    tokens: proc_macro2::TokenStream,
    attrs: &[Attribute],
) -> proc_macro2::TokenStream {
    let cfgs: Vec<_> = (attrs.iter())
        .filter(|attr| attr.path().is_ident("cfg") || attr.path().is_ident("cfg_attr"))
        .collect();
    if cfgs.is_empty() {
        return tokens;
    }
    quote! {
        #(#cfgs)*
        #tokens
    }
}

/// Collect the `#[doc = "..."]` attributes to assign to typed documentation
pub(crate) fn collect_docs(attrs: &[Attribute]) -> Option<String> {
    let docs: Vec<_> = (attrs.iter())
        .filter(|attr| attr.path().is_ident("doc"))
        .collect();

    if docs.is_empty() {
        return None;
    }

    Some(
        docs.into_iter()
            .filter_map(|a| {
                let Meta::NameValue(name) = &a.meta else {
                    return None;
                };
                let syn::Expr::Lit(lit) = &name.value else {
                    return None;
                };
                let syn::Lit::Str(name) = &lit.lit else {
                    return None;
                };
                Some(name.value().trim().to_string())
            })
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

pub fn derive(input: &DeriveInput) -> TokenStream {
    let type_name = &input.ident;

    let named_fields: Option<&FieldsNamed> = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => Some(fields),
            Fields::Unnamed(_) | Fields::Unit => None,
        },
        Data::Enum(_) => None,
        Data::Union(_) => {
            return Error::new_spanned(
                &input,
                "`#[derive(TypedUserData)]` cannot be applied to unions",
            )
            .to_compile_error()
            .into();
        }
    };

    let has_generics = !input.generics.params.is_empty();
    if has_generics {
        return Error::new_spanned(
            &input.generics,
            "`#[derive(TypedUserData)]` does not support generic type parameters. Wrap the generic type in a concrete newtype instead."
        )
        .to_compile_error()
        .into();
    }

    let mut field_registrations = Vec::new();

    if let Some(docs) = collect_docs(&input.attrs) {
        field_registrations.push(quote! { registry.add(#docs); });
    }

    if let Some(fields) = &named_fields {
        for field in &fields.named {
            let field_name = field.ident.as_ref().unwrap();
            let lua_attr = match parse_attrs(&field.attrs, validate_field_attr) {
                Ok(v) => v,
                Err(e) => return e.to_compile_error().into(),
            };

            if lua_attr.skip {
                continue;
            }

            let lua_name = lua_attr.name_or_default(field_name);
            let (has_get, has_set) = if lua_attr.get || lua_attr.set {
                (lua_attr.get, lua_attr.set)
            } else {
                (true, true)
            };

            if has_get {
                let tokens = quote! {
                    registry.add_field_method_get(#lua_name, |_lua, this| Ok(this.#field_name.clone()));
                };
                field_registrations.push(with_cfg(tokens, &field.attrs));
            }
            if has_set {
                let tokens = quote! {
                    registry.add_field_method_set(#lua_name, |_lua, this, val| {
                        this.#field_name = val;
                        Ok(())
                    });
                };
                field_registrations.push(with_cfg(tokens, &field.attrs));
            }
        }
    }

    let registration_type_name = format_ident!("__MluaTypedUserDataRegistration_{type_name}");
    let registration_fields_fn_name = format_ident!("__mlua_register_{type_name}_fields");
    let registration_fields_fn_name_unwrapped =
        format_ident!("__mlua_register_{type_name}_fields_unwrapped");
    let registration_fields_fn_name_wrapped =
        format_ident!("__mlua_register_{type_name}_fields_wrapped");

    let typed_impl = typed::derive(&input);

    quote! {
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        struct #registration_type_name {
            register: fn(&mut ::mlua_extras::typed::registry::TypedUserDataRegistry<#type_name>),
            register_wrapped: for<'ctx> fn(&mut ::mlua_extras::typed::registry::wrapper::TypedUserDataRegistry<'ctx, ::mlua_extras::mlua::userdata::UserDataRegistry<#type_name>>),
        }

        ::mlua_extras::mlua::__inventory::collect!(#registration_type_name);

        #[allow(non_snake_case)]
        fn #registration_fields_fn_name<T: ::mlua_extras::typed::TypedDataDocumentation<#type_name> + ::mlua_extras::typed::TypedDataFields<#type_name>>(registry: &mut T) {
            use ::mlua_extras::typed::{TypedDataDocumentation as _, TypedDataFields as _};
            #(#field_registrations)*
        }

        #[allow(non_snake_case)]
        fn #registration_fields_fn_name_unwrapped(registry: &mut ::mlua_extras::typed::registry::TypedUserDataRegistry<#type_name>) {
            #registration_fields_fn_name(registry);
        }

        #[allow(non_snake_case)]
        fn #registration_fields_fn_name_wrapped<'ctx>(registry: &mut ::mlua_extras::typed::registry::wrapper::TypedUserDataRegistry<'ctx, ::mlua_extras::mlua::userdata::UserDataRegistry<#type_name>>) {
            #registration_fields_fn_name(registry);
        }

        ::mlua_extras::mlua::__inventory::submit! {
            #registration_type_name {
                register: #registration_fields_fn_name_unwrapped,
                register_wrapped: #registration_fields_fn_name_wrapped,
            }
        }

        #typed_impl

        impl ::mlua_extras::typed::TypedUserData for #type_name {
            fn register(registry: &mut ::mlua_extras::typed::registry::TypedUserDataRegistry<Self>) {
                for item in ::mlua_extras::mlua::__inventory::iter::<#registration_type_name> {
                    (item.register)(registry);
                }
            }
        }

        impl ::mlua_extras::mlua::userdata::UserData for #type_name {
            fn register(registry: &mut ::mlua_extras::mlua::userdata::UserDataRegistry<Self>) {
                let mut registry = ::mlua_extras::typed::registry::wrapper::TypedUserDataRegistry::new(registry);
                for item in ::mlua_extras::mlua::__inventory::iter::<#registration_type_name> {
                    (item.register_wrapped)(&mut registry);
                }
            }
        }
    }
}
