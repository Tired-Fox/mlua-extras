#[cfg(feature="mlua")]
pub mod typed;
#[cfg(feature="mlua")]
pub mod extras;

#[cfg(feature="mlua")]
pub use mlua;

#[cfg(feature="derive")]
pub use mlua_extras_derive::{Typed, UserData};

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
