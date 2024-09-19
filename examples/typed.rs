use mlua::{Lua, UserDataMethods, Value};
use mlua_extras::{require::Import, typed::{TypeGenerator, TypedDataFields, TypedDataMethods, TypedFunction, TypedUserData}, LuaExtras, UserData};

#[derive(Default, UserData)]
struct Example {
    data: Option<String>,
}

impl TypedUserData for Example {
    fn add_documentation<F: mlua_extras::typed::TypedDataDocumentation<Self>>(docs: &mut F) {
        docs.add("This is a doc comment section for the overall type");
    }

    fn add_fields<'lua, F: TypedDataFields<'lua, Self>>(fields: &mut F) {
        fields
            .document("Example data")
            .add_field_method_get_set("data",
                |_lua, this| {
                    Ok(this.data.clone())
                },
                |_lua, this, data: String| {
                    this.data.replace(data);
                    Ok(())
                }
            );
    }

    fn add_methods<'lua, T: TypedDataMethods<'lua, Self>>(methods: &mut T) {
        methods
            .document("print the example data")
            .add_method("print", |_lua, this, _: ()| {
                match this.data.as_ref() {
                    Some(v) => println!("{v:?}"),
                    None => println!("nil")
                }
                Ok(())
            })
    }
}

fn main() -> mlua::Result<()> {
    let lua = Lua::new();

    lua.set_global_function("hello", |_lua, name: String| {
        println!("Hello, {name}");
        Ok(())
    })?;

    let hello = lua.require::<TypedFunction<String, ()>>("hello")?;
    hello.call("steve".into())?;

    let mut gen = TypeGenerator::default();
    <Example as TypedUserData>::add_documentation(&mut gen);
    <Example as TypedUserData>::add_fields(&mut gen);
    <Example as TypedUserData>::add_methods(&mut gen);
    println!("{gen:#?}");

    lua.set_global("example", Example::default())?;

    if let Err(err) = lua.load(r#"
example:print()
print(example.data)
example.data = "Some Data"
example:print()
print(example.data)
"#).set_name("#test").eval::<Value>() {
        eprintln!("{err}");
    }

    Ok(())
}
