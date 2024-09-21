use mlua::{Function, Lua, Table, UserData, Value, Variadic};
use mlua_extras::{LuaExtras, Require};

struct MyModule;
impl UserData for MyModule {
    fn add_fields<'lua, F: mlua::prelude::LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field("data", "Some Data");
    }

    fn add_methods<'lua, M: mlua::prelude::LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function("print", |_lua, values: Variadic<Value>| {
            println!(
                "{}",
                values
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<mlua::Result<Vec<_>>>()?
                    .join(" ")
            );
            Ok(())
        });
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

    // Get a value in a nested module/table (trait LuaExtras)
    let table = lua.require::<Table>("table")?;
    // Also works with regular tables (trait Require)
    let _unpack = table.require::<Function>("unpack")?;

    // Import a module into lua's global scope. This is just a UserData
    lua.set_global("mymodule", MyModule)?;

    {
        // Importing also works with tables given a lua context
        let temp = lua.create_table()?;
        temp.set("mymodule", MyModule)?;
    }

    if let Err(err) = lua.load(CODE).eval::<Value>() {
        eprintln!("{err}");
    }

    Ok(())
}
