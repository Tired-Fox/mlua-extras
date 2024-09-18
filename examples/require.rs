use mlua::{Function, Lua, LuaOptions, StdLib, Table, Value, Variadic};
use mlua_extras::require::{Import, Module, TableImport};

struct TableExtras;
impl Module for TableExtras {
    fn extend(lua: &Lua, table: &mlua::Table) -> Result<(), mlua::Error> {
        table.set(
            "keys",
            lua.create_function(|_: &mlua::Lua, this: Table| {
                this.pairs::<Value, Value>()
                    .map(|pair| {
                        let pair = pair?;
                        Ok(pair.0)
                    })
                    .collect::<mlua::Result<Vec<_>>>()
            })?,
        )?;

        table.set(
            "values",
            lua.create_function(|_: &mlua::Lua, this: Table| {
                this.pairs::<Value, Value>()
                    .map(|pair| {
                        let pair = pair?;
                        Ok(pair.1)
                    })
                    .collect::<mlua::Result<Vec<_>>>()
            })?,
        )?;

        Ok(())
    }
}

struct MyModule;
impl Module for MyModule {
    fn extend(lua: &Lua, table: &Table) -> mlua::Result<()> {
        // Add a print function to the module
        table.set(
            "print",
            lua.create_function(|_, values: Variadic<Value>| {
                println!(
                    "{}",
                    values
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<mlua::Result<Vec<_>>>()?
                        .join(" ")
                );
                Ok(())
            })?,
        )?;

        // Add a string to the module
        table.set("data", "Some data")?;

        Ok(())
    }
}

const CODE: &str = r#"data = {
    first = "key",
    second = "value"
}

print("KEYS: ", table.unpack(table.keys(data)))
print("VALUES: ", table.unpack(table.values(data)))

mymodule.print(mymodule.data)
"#;

fn main() -> mlua::Result<()> {
    let lua = Lua::new();

    // Get a value in a nested module/table
    let table = lua.require::<Table>("table")?;
    // Also works with regular tables
    let _unpack = table.require::<Function>("unpack")?;

    // Extend an existing table with a module's contents
    table.extend::<TableExtras>(&lua)?;
    // TableExtras::extend(&lua, table)?;

    // Import a module into lua's global scope
    lua.import::<MyModule>("mymodule")?;

    {
        // Importing also works with tables given a lua context
        let temp = lua.create_table()?;
        temp.import::<MyModule>(&lua, "mymodule")?;
    }

    if let Err(err) = lua.load(CODE).eval::<Value>() {
        eprintln!("{err}");
    }

    Ok(())
}
