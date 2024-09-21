use mlua::{Error, FromLua, Table};

/// Adds a similar syntax to tables as lua's `require` function.
///
/// # Example
///
/// ```
/// use mlua_extras::{
///     mlua::{Lua, Function},
///     extras::Require,
/// };
///
/// let lua = Lua::new();
///
/// let unpack = lua.require::<Function>("table.unpack")?;
/// ```
///
/// Would be the same as
///
/// ```lua
/// local unpack = require("table").unpack
/// ```
pub trait Require<'lua> {
    /// Fetch a nested lua value from the table
    fn require<R: FromLua<'lua>>(&'lua self, path: impl AsRef<str>) -> mlua::Result<R>;
}

impl<'lua> Require<'lua> for Table<'lua> {
    fn require<R: FromLua<'lua>>(&'lua self, path: impl AsRef<str>) -> mlua::Result<R> {
        let segments = path
            .as_ref()
            .split('.')
            .filter_map(|v| (!v.trim().is_empty()).then_some(v.trim()))
            .collect::<Vec<_>>();

        let mut module = self.clone();
        if !segments.is_empty() {
            for seg in &segments[..segments.len() - 1] {
                module = module.get::<_, Table>(*seg)?;
            }
        }

        match segments.last() {
            Some(seg) => module.get::<_, R>(*seg),
            None => Err(Error::runtime(format!(
                "module not found: {:?}",
                path.as_ref()
            ))),
        }
    }
}
