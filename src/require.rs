use mlua::{Error, FromLua, Lua, Table};

pub trait Require<'lua> {
    /// Fetch a nested table from the current scope
    fn require<R: FromLua<'lua>>(&'lua self, path: impl AsRef<str>) -> mlua::Result<R>;
}

impl<'lua> Require<'lua> for Table<'lua> {
    fn require<R: FromLua<'lua>>(&'lua self, path: impl AsRef<str>) -> mlua::Result<R> {
        let segments = path.as_ref()
            .split('.')
            .filter_map(|v| (!v.trim().is_empty()).then_some(v.trim()))
            .collect::<Vec<_>>();

        let mut module = self.clone();
        if !segments.is_empty() {
            for seg in &segments[..segments.len()-1] {
                module = module.get::<_, Table>(*seg)?;
            }
        }

        match segments.last() {
            Some(seg) => module.get::<_, R>(*seg),
            None => Err(Error::runtime(format!("module not found: {:?}", path.as_ref())))
        }
    }
}

pub trait Module {
    /// Extend an existing table with the modules contents
    fn extend(lua: &Lua, table: &Table) -> mlua::Result<()>;

    /// Create the module and return it
    ///
    /// # Returns
    ///
    /// [`mlua::Table`] containing the module
    fn require(lua: &Lua) -> mlua::Result<Table>  {
        let table = lua.create_table()?;
        Self::extend(lua, &table)?;
        Ok(table)
    }
}
