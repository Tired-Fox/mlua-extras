//!# MLua Typed Derive

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro_error::{proc_macro_error, abort, emit_error, emit_warning};
use syn::spanned::Spanned;
use venial::{parse_item, Item};

#[proc_macro_error]
#[proc_macro_derive(UserData)]
pub fn derive_user_data(input: TokenStream) -> TokenStream {
    let input = TokenStream2::from(input);
    let name = match parse_item(input.clone()) {
        Ok(Item::Struct(struct_type)) => {
            struct_type.name.clone()
        },
        Ok(Item::Enum(enum_type)) => {
            enum_type.name.clone()
        },
        Err(err) => abort!(err.span(), "{}", err),
        _ => abort!(input.span(), "only `struct` and `enum` types are supported for TypedUserData")
    };

    quote!(
        impl mlua::UserData for #name {
            fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(fields: &mut F) {
                let mut wrapper = mlua_extras::typed::WrappedGenerator::new(fields);
                <#name as mlua_extras::typed::TypedUserData>::add_fields(&mut wrapper);
            }

            fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
                let mut wrapper = mlua_extras::typed::WrappedGenerator::new(methods);
                <#name as mlua_extras::typed::TypedUserData>::add_methods(&mut wrapper);
            }
        }
    ).into()
}
