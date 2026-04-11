#[macro_use]
extern crate quote;

mod lua_user_data;
mod methods_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
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
            fn add_fields<F: mlua_extras::mlua::UserDataFields<Self>>(fields: &mut F) {
                let mut wrapper = mlua_extras::typed::WrappedBuilder::new(fields);
                <#name as mlua_extras::typed::TypedUserData>::add_fields(&mut wrapper);
            }

            fn add_methods<M: mlua_extras::mlua::UserDataMethods<Self>>(methods: &mut M) {
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
            let label = name.to_string();
            quote!(
                impl mlua_extras::typed::Typed for #name {
                    fn ty() -> mlua_extras::typed::Type {
                        mlua_extras::typed::Type::class(mlua_extras::typed::TypedClassBuilder::new::<#name>())
                    }

                    fn as_param() -> mlua_extras::typed::Param {
                        mlua_extras::typed::Param {
                            doc: None,
                            name: None,
                            ty: mlua_extras::typed::Type::named(#label),
                        }
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
            quote!(
                impl mlua_extras::typed::Typed for #name {
                    fn ty() -> mlua_extras::typed::Type {
                        mlua_extras::typed::Type::r#enum(
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

/// Derive macro that generates a `TypedUserData` implementation from struct fields.
///
/// Each named field is automatically exposed to Lua as a read/write property.
/// Use `#[mlua_extras(...)]` attributes on fields to control access:
///
/// - `#[mlua_extras(skip)]` — field is not exposed to Lua
/// - `#[mlua_extras(readonly)]` — getter only
/// - `#[mlua_extras(writeonly)]` — setter only
/// - `#[mlua_extras(rename = "lua_name")]` — use a different name in Lua
///
/// Doc comments on fields are forwarded to the type metadata system.
///
/// This also generates the `mlua::UserData` impl, so you do not need to
/// separately derive `UserData`.
///
/// # Example
///
/// ```ignore
/// #[derive(Clone, TypedUserData)]
/// struct Player {
///     /// The player's display name
///     name: String,
///     health: f64,
///     #[mlua_extras(skip)]
///     internal_id: u64,
///     #[mlua_extras(readonly)]
///     score: i32,
///     #[mlua_extras(rename = "pos_x")]
///     position_x: f64,
/// }
/// ```
///
/// Optionally combine with [`macro@typed_user_data_impl`] to also register methods.
/// See `tests/lua_user_data.rs` for more exhaustive examples.
#[proc_macro_error]
#[proc_macro_derive(TypedUserData, attributes(mlua_extras))]
pub fn derive_typed_user_data(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    lua_user_data::derive(input).into()
}

/// Attribute macro that registers methods from an `impl` block for use in Lua.
///
/// Place on an `impl` block for a type that derives [`TypedUserData`](macro@TypedUserData).
/// Annotate individual methods with `#[method]` or `#[metamethod(...)]`.
///
/// # Method attributes
///
/// - `#[method]` — register as a regular Lua method/function
/// - `#[method(rename = "lua_name")]` — register under a different Lua name
/// - `#[metamethod(ToString)]` — register as a metamethod (any `mlua::MetaMethod` variant)
/// - `#[metamethod("__custom")]` — register a custom-named metamethod
///
/// # Receiver handling
///
/// - `&self` → `add_method`
/// - `&mut self` → `add_method_mut`
/// - no `self` → `add_function`
/// - `async fn` with `&self` → `add_async_method`
/// - `async fn` no `self` → `add_async_function`
///
/// # Optional `lua` parameter
///
/// If the first non-self parameter is named `lua`, it receives the Lua context
/// from the closure and is not part of the Lua-side argument list.
///
/// # Return types
///
/// - `-> Result<T, E>` where `E: Into<mlua::Error>` — fallible; the error is
///   converted via `.into()`. This includes `mlua::Result<T>`, `mlua::Error`,
///   `anyhow::Error` (with the mlua `anyhow` feature), `std::io::Error`, and
///   any type implementing mlua's `ExternalError` trait.
/// - `-> T` — infallible, wrapped in `Ok(...)`
/// - no return / `-> ()` — returns `Ok(())`
///
/// # Example
///
/// ```ignore
/// #[derive(Clone, TypedUserData)]
/// struct Counter { value: i64 }
///
/// #[mlua_extras::typed_user_data_impl]
/// impl Counter {
///     #[method]
///     fn get(&self) -> i64 { self.value }
///
///     #[method]
///     fn increment(&mut self) { self.value += 1; }
///
///     #[method]
///     fn create_table(&self, lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
///         lua.create_table()
///     }
///
///     #[metamethod(ToString)]
///     fn to_string(&self) -> String { format!("Counter({})", self.value) }
///
///     #[method]
///     async fn fetch(&self, url: String) -> mlua::Result<String> {
///         Ok(format!("fetched: {url}"))
///     }
/// }
/// ```
///
/// Methods without `#[method]` or `#[metamethod(...)]` are left as normal Rust
/// methods, callable from Rust but not registered with Lua.
///
/// See `tests/lua_user_data.rs` for more exhaustive examples.
#[proc_macro_error]
#[proc_macro_attribute]
pub fn typed_user_data_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(item as syn::ItemImpl);
    methods_macro::methods_impl(item).into()
}
