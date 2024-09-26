#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use proc_macro_error::{proc_macro_error, abort};
use syn::spanned::Spanned;
use venial::{parse_item, Fields, Item};

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
        impl mlua_extras::mlua::UserData for #name {
            fn add_fields<'lua, F: mlua_extras::mlua::UserDataFields<'lua, Self>>(fields: &mut F) {
                let mut wrapper = mlua_extras::typed::WrappedBuilder::new(fields);
                <#name as mlua_extras::typed::TypedUserData>::add_fields(&mut wrapper);
            }

            fn add_methods<'lua, M: mlua_extras::mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
                let mut wrapper = mlua_extras::typed::WrappedBuilder::new(methods);
                <#name as mlua_extras::typed::TypedUserData>::add_methods(&mut wrapper);
            }
        }
    ).into()
}

#[proc_macro_error]
#[proc_macro_derive(Typed, attributes(typed))]
pub fn derive_typed(input: TokenStream) -> TokenStream {
    let input = TokenStream2::from(input);
    match parse_item(input.clone()) {
        Ok(Item::Struct(struct_type)) => {
            let name = struct_type.name.clone();
            let value = syn::LitStr::new(name.to_string().as_str(), Span::call_site());
            quote!(
                impl mlua_extras::typed::Typed for #name {
                    fn ty() -> mlua_extras::typed::Type {
                        mlua_extras::typed::Type::Single(#value.into())
                    }
                }
            )
        },
        Ok(Item::Enum(enum_type)) => {
            let variants = enum_type.variants
                .iter()
                .map(|(variant, _punc)| {
                    let name = format!("\"{}\"", variant.name);
                    match &variant.fields {
                        Fields::Unit => quote!{ mlua_extras::typed::Type::Single(#name.into()) },
                        Fields::Tuple(tf) => {
                            let tuple_values = tf.fields.iter().map(|(field, _)| {
                                let ty = field.ty.clone();
                                quote!{ <#ty as mlua_extras::typed::Typed>::ty() }
                            }).collect::<Vec<_>>();

                            if tuple_values.len() == 1 {
                                let first = tuple_values.first().unwrap();
                                quote!{ #first }
                            } else {
                                quote!{ mlua_extras::typed::Type::Tuple(Vec::from([
                                        #(#tuple_values,)*
                                ])) }
                            }
                        },
                        Fields::Named(named) => {
                            let tuple_values = named.fields.iter().map(|(field, _)| {
                                let name = field.name.to_string();
                                let ty = field.ty.clone();
                                quote!{ (mlua_extras::typed::Index::from(#name), <#ty as mlua_extras::typed::Typed>::ty()) }
                            }).collect::<Vec<_>>();
                            quote!{ mlua_extras::typed::Type::Table(std::collections::BTreeMap::from([
                                    #(#tuple_values,)*
                            ])) }
                        }
                    }
                    
                })
                .collect::<Vec<_>>();

            // TODO: This should be a union alias
            let name = enum_type.name.clone();
            let value = name.to_string();
            quote!(
                impl mlua_extras::typed::Typed for #name {
                    fn ty() -> mlua_extras::typed::Type {
                        mlua_extras::typed::Type::r#enum(
                            #value,
                            [ #(#variants,)* ]
                        )
                    }
                }
            )
        },
        Err(err) => abort!(err.span(), "{}", err),
        _ => abort!(input.span(), "only `struct` and `enum` types are supported for Typed")
    }.into()
}
