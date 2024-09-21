/// Write a lua function similar to Rust's syntax
///
/// # Example
///
/// ```
/// use mlua::Lua;
///
/// let lua = Lua::new();
/// lua.create_function(|lua, ()| Ok(()))
/// ```
///
/// vs
///
/// ```
/// use mlua::Lua;
///
/// let lua = Lua::new();
/// function! {
///     lua fn name(lua) {
///         Ok(())
///     }
/// }
/// ```
///
/// It can also be used to asssign functions to nested tables. This requires the `LuaExtras` crate
/// when you start with lua as the source, and the `Require` trait when using any other table as
/// the source.
///
/// ```
/// use mlua::{Lua, Table};
///
/// let lua = Lua::new();
/// lua.globals().get::<_, Table>("nested")?.set("name", lua.create_function(|lua, ()| Ok(()))?)?;
///
/// let nested = lua.globals().get::<_, Table>("deep")?.get::<_, Table>("nested");
/// nested.set("name", lua.create_function(|lua, ()| Ok(())))?;
/// ```
///
/// vs
///
/// ```
/// use mlua::{Lua, Table};
/// use mlua_extras::{LuaExtras, Require};
///
/// let lua = Lua::new();
/// function! {
///     lua fn lua::nested.name(lua) {
///         Ok(())
///     }
/// }
///
/// let nested = lua.globals().get::<_, Table>("deep")?;
/// function! {
///     lua fn deep::nested.name(lua) {
///         Ok(())
///     }
/// }
/// ```
#[macro_export]
macro_rules! function {
    {
        $(#[$($attr: tt)*])*
        $lua: ident fn $name: ident(
            $l: ident $(: $lty: ty)?
            $(, $arg: ident : $aty: ty)*
        ) $(-> $ret: ty)? {
            $($body: tt)*
        }
    } => {
        $lua.create_function(|$l $(: $lty)?, ($($arg,)*): ($($aty,)*)| $(-> $ret)? {
            $($body)*
        })
    };
    {
        $(#[$($attr: tt)*])*
        $lua: ident fn $source: ident $(::$inner: ident)* . $name: ident(
            $l: ident $(: $lty: ty)?
            $(, $arg: ident : $aty: ty)*
        ) $(-> $ret: ty)? {
            $($body: tt)*
        }
    } => {
        {
            match $source.require::<mlua::Table>(stringify!($($inner.)*)) $(-> $ret: ty)? {
                Ok(table) => match $lua.create_function(|$l$(: $lty)?, ($($arg,)*): ($($aty,)*)| $(-> $ret)? {
                    $($body)*
                }) {
                    Ok(fun) => table.set(stringify!($name), fun),
                    Err(err) => Err(err)
                }
                Err(err) => Err(err)
            }
        }
    };
}
