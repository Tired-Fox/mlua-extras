use std::path::PathBuf;

use mlua_extras::{
    mlua::{self, FromLua, Lua, LuaSerdeExt, MetaMethod, UserDataMethods, Value, Variadic},
    typed::{
        generator::{Definition, Definitions, DefinitionFileGenerator},
        TypedDataFields, TypedDataMethods, TypedUserData,
    },
    extras::LuaExtras,
    Typed, UserData,
};
use serde::Deserialize;

#[derive(Default, Debug, Clone, Copy, Typed, Deserialize)]
enum SystemColor {
    #[default]
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Cyan,
    Magenta,
    White,
}

#[derive(Debug, Clone, Copy, Typed, UserData, Deserialize)]
#[serde(untagged)]
enum Color {
    System(SystemColor),
    Xterm(u8),
    Rgb(u8, u8, u8),
}

impl Default for Color {
    fn default() -> Self {
        Color::System(SystemColor::default())
    }
}

impl Color {
    pub fn background_ansi(&self) -> String {
        match self {
            Self::System(system) => match system {
                SystemColor::Black => "\x1b[40m".into(),
                SystemColor::Red => "\x1b[41m".into(),
                SystemColor::Green => "\x1b[42m".into(),
                SystemColor::Yellow => "\x1b[43m".into(),
                SystemColor::Blue => "\x1b[44m".into(),
                SystemColor::Magenta => "\x1b[45m".into(),
                SystemColor::Cyan => "\x1b[46m".into(),
                SystemColor::White => "\x1b[47m".into(),
            },
            Self::Xterm(xterm) => format!("\x1b[48;5;{xterm}m"),
            Self::Rgb(r, g, b) => format!("\x1b[48;2;{r};{g};{b}m"),
        }
    }
}

impl TypedUserData for Color {
    fn add_documentation<F: mlua_extras::typed::TypedDataDocumentation<Self>>(docs: &mut F) {
        docs.add("Representation of a color");
    }

    fn add_methods<'lua, T: TypedDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_meta_method(MetaMethod::ToString, |_lua, this, _: ()| {
            Ok(format!("{this:?}"))
        });
    }
}

impl<'lua> FromLua<'lua> for Color {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> mlua::prelude::LuaResult<Self> {
        match value {
            Value::UserData(data) => data.borrow::<Self>().map(|v| *v),
            // Use serde deserialize if not userdata
            other => lua.from_value(other),
        }
    }
}

#[derive(Debug, Clone, Copy, UserData, Typed, Deserialize)]
struct Example {
    color: Color,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            color: Color::Rgb(30, 132, 129),
        }
    }
}

impl<'lua> FromLua<'lua> for Example {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> mlua::prelude::LuaResult<Self> {
        match value {
            Value::UserData(data) => data.borrow::<Self>().map(|v| *v),
            other => lua.from_value(other),
        }
    }
}

impl TypedUserData for Example {
    fn add_documentation<F: mlua_extras::typed::TypedDataDocumentation<Self>>(docs: &mut F) {
        docs.add("This is a doc comment section for the overall type");
    }

    fn add_fields<'lua, F: TypedDataFields<'lua, Self>>(fields: &mut F) {
        fields
            .document("Example complex type")
            .add_field_method_get_set(
                "color",
                |_lua, this| Ok(this.color),
                |_lua, this, clr: Color| {
                    this.color = clr;
                    Ok(())
                },
            );
    }

    fn add_methods<'lua, T: TypedDataMethods<'lua, Self>>(methods: &mut T) {
        methods.document("print all items").add_function(
            "printAll",
            |_lua, all: Variadic<String>| {
                println!(
                    "{}",
                    all.iter().map(|v| v.as_str()).collect::<Vec<_>>().join(" ")
                );
                Ok(())
            },
        );

        methods.add_meta_method(MetaMethod::ToString, |_lua, this, ()| {
            Ok(format!("{this:?}"))
        });
    }
}

fn main() -> mlua::Result<()> {
    let lua = Lua::new();

    // ===== Setup Lua Engine =====

    lua.set_global("example", Example::default())?;

    lua.set_global_function("greet", |_lua, name: String| {
        println!("Hello, {name}");
        Ok(())
    })?;

    lua.set_global_function("printColor", |_lua, color: Color| {
        println!("{}      \x1b[0m {color:?}", color.background_ansi());
        Ok(())
    })?;

    // ===== Generate Types and Definition Files =====

    let definitions = Definitions::start()
        .define("init", Definition::start()
            .register_enum::<SystemColor>()?
            .register_enum::<Color>()?
            .register_class::<Example>()
            .value::<Example, _>("example")
            .function::<String, (), _>("greet", ())
            .function::<Color, (), _>("printColor", ())
        )
        .finish();

    let types_path = PathBuf::from("examples/types");
    if !types_path.exists() {
        std::fs::create_dir_all(&types_path).unwrap();
    }

    let gen = DefinitionFileGenerator::new(definitions);
    for (name, writer) in gen.iter() {
        println!("==== Generated \x1b[1;33mexample/types/{name}\x1b[0m ====");
        writer.write_file(types_path.join(name)).unwrap();
    }
    println!();

    // ===== Run user defined file... This will default if file doesn't exist =====
    let default = r#"
example.printAll("Some", "text", "printed", "with", "a", "single", "space")
printColor(example.color)
printColor({ 30, 129, 20 })
printColor(211)
printColor("Blue")
"#;

    let user_file = PathBuf::from("examples/typed.lua");

    if user_file.exists() {
        if let Err(err) = lua.load(user_file).eval::<Value>() {
            eprintln!("{err}");
        }
    } else {
        println!(
            "\x1b[1;36mNOTE\x1b[22;39m This is the default example lua code for the typed example"
        );
        println!(
            "\x1b[1;36mNOTE\x1b[22;39m create a file at `examles/typed.lua` to run your own code. \
        LuaLS should pull in the generated `examples/types/init.d.lua` automatically"
        );
        println!();

        if let Err(err) = lua.load(default).eval::<Value>() {
            eprintln!("{err}");
        }
    }

    Ok(())
}
