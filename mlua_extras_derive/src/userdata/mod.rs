use proc_macro2::TokenStream;
use syn::{Attribute, Data, DeriveInput, Error, Fields, FieldsNamed};

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
    let registration_fields_fn_name_wrapped = format_ident!("__mlua_register_{type_name}_fields_wrapped");

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
        fn #registration_fields_fn_name(registry: &mut ::mlua_extras::typed::registry::TypedUserDataRegistry<#type_name>) {
            use ::mlua_extras::typed::TypedDataFields as _;
            #(#field_registrations)*
        }

        #[allow(non_snake_case)]
        fn #registration_fields_fn_name_wrapped<'ctx>(registry: &mut ::mlua_extras::typed::registry::wrapper::TypedUserDataRegistry<'ctx, ::mlua_extras::mlua::userdata::UserDataRegistry<#type_name>>) {
            use ::mlua_extras::typed::TypedDataFields as _;
            #(#field_registrations)*
        }

        ::mlua_extras::mlua::__inventory::submit! {
            #registration_type_name {
                register: #registration_fields_fn_name,
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
