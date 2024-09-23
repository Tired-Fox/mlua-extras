use mlua_extras::{extras::Module, typed::{TypedModule, TypedModuleBuilder, TypedModuleFields, TypedModuleMethods}};

struct Nested;
impl TypedModule for Nested {
    fn add_methods<'lua, M: TypedModuleMethods<'lua>>(methods: &mut M) -> mlua::Result<()> {
        methods
            .document("Print hello to the name passed in")
            .add_function_with(
                "hello",
                |_lua, name: String| {
                    println!("Hello, {name}!");
                    Ok(())
                },
                |func| {
                    func.param(0, |param| { param.set_name("name").set_doc("Name of the person to greet"); });
                })
    }
}

struct Test;
impl TypedModule for Test {
    fn add_fields<'lua, F: TypedModuleFields<'lua>>(fields: &mut F) -> mlua::Result<()> {
        fields
            .document("Name to use in with the greeting method")
            .add_field("name", "mlua-extras")?;
        fields
            .document("Nested module")
            .add_module::<Nested>("nested")?;

        Ok(())
    }

    fn add_methods<'lua, M: TypedModuleMethods<'lua>>(methods: &mut M) -> mlua::Result<()> {
        methods
            .document("Greet the table's `name` field")
            // TODO: Add `add_method_with` and associated methods to add docs for params and
            // returns right away
            .add_method("greet", |_lua, this, ()| {
                let name = this.get::<_, String>("name")?;
                println!("Hello, {name}!");
                Ok(())
            })?;

        Ok(())
    }
}

fn main() -> mlua::Result<()> {
    let lua = mlua::Lua::new();

    lua.globals().set("test", Test::module())?;

    let test_mod = TypedModuleBuilder::new::<Test>();

    println!("{test_mod:#?}");

    if let Err(err) = lua.load(r#"
test.name = "Zachary"
test:greet()
test.nested.hello("Zachary")
"#).eval::<mlua::Value>() {
        eprintln!("{err}");
    }

    Ok(())
}
