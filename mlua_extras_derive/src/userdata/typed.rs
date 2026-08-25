use proc_macro2::TokenStream;
use syn::{ext::IdentExt, Data};

pub fn derive(input: &syn::DeriveInput) -> TokenStream {
    let name = &input.ident;
    match &input.data {
        Data::Struct(_) => {
            let label = name.to_string();
            quote!(
                impl mlua_extras::typed::Typed for #name {
                    fn ty() -> mlua_extras::typed::Type {
                        mlua_extras::typed::Type::Class(Box::new(mlua_extras::typed::TypedUserDataRegistry::new::<#name>().build()))
                    }

                    fn as_param() -> mlua_extras::typed::Type {
                        mlua_extras::typed::Type::named(#label)
                    }

                    fn as_return() -> mlua_extras::typed::Type {
                        mlua_extras::typed::Type::named(#label)
                    }
                }
            )
        }
        Data::Enum(enum_type) => {
            let label = name.to_string();
            let variants = enum_type
                .variants
                .iter()
                .map(|v| format!("\"{}\"", v.ident.unraw().to_string()));
            quote!(
                impl mlua_extras::typed::Typed for #name {
                    fn ty() -> mlua_extras::typed::Type {
                        mlua_extras::typed::Type::Union(Vec::from([
                            #(mlua_extras::typed::Type::Single(std::borrow::Cow::Borrowed(#variants.into())),)*
                        ]))
                    }

                    fn as_param() -> mlua_extras::typed::Type {
                        mlua_extras::typed::Type::named(#label)
                    }

                    fn as_return() -> mlua_extras::typed::Type {
                        mlua_extras::typed::Type::named(#label)
                    }
                }
            )
        }
        _ => proc_macro_error::abort!(
            input,
            "only `struct` and `enum` types are supported for Typed"
        ),
    }
}
