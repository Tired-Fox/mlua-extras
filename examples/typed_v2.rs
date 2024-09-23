use std::io::stdout;

use mlua_extras::typed::generator::{Definition, DefinitionFileGenerator, Definitions};

fn main() {
    let defs = Definitions::start()
        .define("init", Definition::start()
            .function::<String, ()>("greet", ())
            .function_with::<String, String, _>("greet", (), |func| {
                func.param(0, |param| { param.set_name("name").set_doc("Name of the person to greet"); });
                func.ret(0, |ret| { ret.set_doc("Formatted greeting using the given name"); });
            })
        )
        .finish();

    for (name, writer) in DefinitionFileGenerator::new(defs).iter() {
        println!("==== {name} ====");
        writer.write(stdout()).unwrap();
    }
}
