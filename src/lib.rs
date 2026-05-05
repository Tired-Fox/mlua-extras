pub mod ser;

#[cfg(feature = "mlua")]
pub mod extras;
#[cfg(feature = "mlua")]
pub mod typed;

#[cfg(feature = "mlua")]
pub use mlua;

#[cfg(feature = "macros")]
pub use mlua_extras_derive::{
    Typed, TypedUserData, UserData, typed_user_data_impl, user_data_impl,
};

#[cfg(feature = "send")]
/// Used by the `send` feature
pub trait MaybeSend: Send {}
#[cfg(feature = "send")]
impl<T: Send> MaybeSend for T {}

#[cfg(not(feature = "send"))]
/// Used by the `send` feature
pub trait MaybeSend {}
#[cfg(not(feature = "send"))]
impl<T> MaybeSend for T {}

#[cfg(feature = "macros")]
#[doc(hidden)]
pub trait __DefaultAutoMethods: Sized {
    fn __auto_add_methods<M>(_m: &mut M) {}
}
#[cfg(feature = "macros")]
impl<T: Sized> __DefaultAutoMethods for T {}

#[cfg(feature = "macros")]
#[doc(hidden)]
pub trait __DefaultAutoFields: Sized {
    fn __auto_add_fields<F>(_f: &mut F) {}
}
#[cfg(feature = "macros")]
impl<T: Sized> __DefaultAutoFields for T {}
