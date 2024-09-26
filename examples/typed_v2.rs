use std::io::stdout;

use mlua::MetaMethod;
use mlua_extras::typed::{generator::{Definition, DefinitionFileGenerator, Definitions}, Type, TypedModule};

struct NestedModule;
impl TypedModule for NestedModule {
    fn documentation() -> Option<String> {
        Some("Nested module".into())
    }
}

struct TestModule;
impl TypedModule for TestModule {
    fn documentation() -> Option<String> {
        Some("Test module documentation".into())
    }

    fn add_fields<'lua, F: mlua_extras::typed::TypedModuleFields<'lua>>(fields: &mut F) -> mlua::Result<()> {
        fields
            .document("Some test data")
            .add_field("data", "Some data")?;

        fields
            .document("Meta field")
            .add_meta_field("__count", 0u32)?;

        fields
            .document("Nested module")
            .add_module::<NestedModule>("nested")?;

        Ok(())
    }

    fn add_methods<'lua, M: mlua_extras::typed::TypedModuleMethods<'lua>>(methods: &mut M) -> mlua::Result<()> {
        methods
            .document("Greetings")
            .add_function_with("greet", |_, _name: String| { Ok(()) }, |func| {
                func.param(0, |param| param.doc("Name of the person to greet").name("name"));
            })?;

        methods
            .document("Convert the test module to a string")
            .add_meta_method(MetaMethod::ToString, |_, _this, ()| {
                Ok(String::new())
            })?;

        Ok(())
    }
}

fn main() {
    let defs = Definitions::start()
        .define("init", Definition::start()
            .module::<TestModule>("test")
            .function::<String, ()>("greet", ())
            .function_with::<String, String, _>("greet", (), |func| {
                func.param(0, |param| param.name("name").doc("Name of the person to greet"));
                func.ret(0, |ret| ret.doc("Formatted greeting using the given name"));
            })
        )
        .finish();

    for (name, writer) in DefinitionFileGenerator::new(defs).iter() {
        println!("==== {name} ====");
        writer.write(stdout()).unwrap();
    }

    println!("{:#?}", Type::string() | "literal" | true | 0usize | [Type::string(), Type::nil(), Type::literal(3)]);
}
