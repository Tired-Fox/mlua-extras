#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;

mod userdata;

/// Generates a [`Typed`](mlua_extras::Typed) implementation from fields.
///
/// Only supports structs and enums.
///
/// This registers the target as a new Lua type that can be used
/// to generate documentation.
///
/// # Structs
///
/// Assigned as a lua `class` with it's registered fields, indexes, methods,
/// functions and their meta variants.
///
/// # Enums
///
/// Assigned as an alias to a union of `class`es where each `class` is a enum variant. This
/// is to best represent rust's use of enums as unions.
#[proc_macro_error]
#[proc_macro_derive(Typed)]
pub fn derive_typed(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    userdata::typed::derive(&input).into()
}

/// Generates a [mlua_extras::typed::TypedUserData] implementation from struct fields.
///
/// Each named and unnamed field is automatically exposed to Lua as a read and/or write property or index.
///
/// Use `#[lua(...)]` attributes to control access and naming:
///
/// - `readonly`: Set the field to only be readable within Lua
/// - `writeonly`: Set the field to only be writable within Lua
/// - `skip`: Ignore generating and exposing the field
/// - `rename`: Rename the field to a string for a named field and a digit for an indexed field
///
/// > Note: `readonly` + `writeonly` together is the same as having neither, the field will be exposed
/// > for both read and write.
///
/// Optionally combine with [`macro@typed_user_data_impl`] to also register methods in a rust like manner.
///
/// # Example
///
/// ```ignore
/// #[derive(Clone, TypedUserData)]
/// struct Player {
///     name: String,
///     health: f64,
///     #[lua(skip)]
///     handle: u64,
///     #[lua(readonly)]
///     score: i32,
///     #[lua(name = "pos_x")]
///     position_x: f64,
/// }
/// ```
///
/// ```ignore
/// #[derive(Clone, TypedUserData)]
/// enum PlayerAction {
///     Idle,
///     Move,
///     Attack,
///     Quit,
/// }
/// ```
#[proc_macro_error]
#[proc_macro_derive(TypedUserData, attributes(lua))]
pub fn derive_typed_user_data(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    userdata::derive(&input).into()
}

/// Attribute macro that registers methods from an `impl` block for use in Lua.
///
/// Used on an `impl` block for a type that derives [`TypedUserData`](macro@TypedUserData), this
/// macro will register methods annotated with `#[lua(method)]`, `#[lua(meta, ...)]`, `#[lua(getter, ...)]`,
/// `#[lua(setter, ...)]`, and `#[lua(field)]` along with const expressions with or without `#[lua(field)]`.
///
/// # Attributes
///
/// - `#[lua(method)]`
///   - `#[lua(method, rename = "name")`: register as a method with the provided name
/// - `#[lua(meta, ...)]`
///   - `#[lua(meta, rename = ToString)]`: register as a metamethod as a [`mlua::MetaMethod`] variant
///   - `#[lua(meta, rename = "__custom")]`: register as a custom named metamethod
/// - `#[lua(getter, ...)]`
///   - `#[lua(getter, rename = "field")]`: register the function as a getter for the named field
/// - `#[lua(setter, ...)]`
///   - `#[lua(setter, "field")]`: register the function as a setter for the named field
/// - `#[lua(field, ...)]`
///   - Applied to a function will call the function once to register a static field
///   - Applied to a `const` expr will register the value as a static field
///   - `#[lua(field, rename="field")]`: register field with the custom name
/// - `#[lua(skip)]`: ignore the function or `const` expr and don't register it
///
/// # Patterns
///
/// - `&self`: registered with `TypedDataMethods::add_method` or `TypedDataMethods::add_meta_method`
/// - `&mut self`: registered with `TypedDataMethods::add_method_mut` or `TypedDataMethods::add_meta_method_mut`
/// - without `self`: registered with `TypedDataMethods::add_function` or `TypedDataMethods::add_meta_function`
/// - `async fn`: registered with the `TypedDataMethods::add_async_*` variant that matches the above arguments
/// - If the first non `self` parameter is `lua` then `&mlua::Lua` is passed to non async methods/functions
///     and `mlua::Lua` is passed into async methods/functions
///
/// # Return
///
/// - `Result<T, E>` where `E: Into<mlua::Error>`: Method is fallible and the error is automatically converted to a [`mlua::Error`].
///     This includes any error type that implements [`mlua::ExternalError`] and any return type that has the name `Result`.
/// - `T`: Method is infallible and is wrapped with `Ok(...)` when registered
/// - `()`: Method is infallible and has no return value. Registration returns `Ok(())`
///
/// > NOTE: All infallible methods must be marked with `#[lua(infallible)]`
///
/// All methods stay as is and stay as regular callable rust functions. Any methods without one of the listed attribute macros will not be registered.
///
/// # Example
///
/// ```ignore
/// #[derive(Clone, TypedUserData)]
/// struct Counter { value: i64 }
///
/// #[typed_user_data_impl]
/// impl Counter {
///     #[lua(field, name = "name")]
///     const NAME: &'static str = "Counter";
///     const COUNT: usize = 10;
///
///     #[lua(field)]
///     fn max() -> i64 {
///         i64::MAX
///     }
///
///     #[lua(field, name = "MIN")]
///     fn min() -> i64 {
///         0
///     }
///
///     #[lua(getter, name = "direction")]
///     fn get_direction(&self) -> String {
///         "west".into()
///     }
///
///     #[lua(setter, name = "direction")]
///     fn set_direction(&mut self, dir: String) {
///         _ = dir;
///     }
///
///     fn get(&self) -> i64 { self.value }
///
///     #[lua(method)]
///     fn increment(&mut self) { self.value += 1 }
///
///     #[method]
///     fn create_table(&self, lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
///         lua.create_table()
///     }
///
///     #[lua(meta, name = "__tostring")]
///     fn to_string(&self) -> String { format!("Counter({})", self.value) }
///
///     #[lua(meta)]
///     fn __tostring(&self) -> String { format!("Counter({})", self.value) }
///
///     // Requires the `async` feature
///     // Must be accessed from lua code with an entry of `mlua::Chunk::eval_async` or `mlua::Chunk::exec_async`
///     async fn fetch(&self, lua: mlua::Lua, url: String) -> mlua::Result<String> {
///         _ = lua;
///         Ok(format!("fetched: {url}"))
///     }
/// }
/// ```
#[proc_macro_error]
#[proc_macro_attribute]
pub fn typeduserdata_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new_spanned(
            proc_macro2::TokenStream::from(attr),
            "`#[typeduserdata_impl]` does not accept arguments",
        )
        .to_compile_error()
        .into();
    }

    let mut input = syn::parse_macro_input!(item as syn::ItemImpl);
    userdata::userdata_impl::derive(&mut input).into()
}
