use std::path::PathBuf;

use mlua_extras::{
    Typed, TypedUserData, extras::LuaExtras, mlua::{self, FromLua, Lua, LuaSerdeExt, MetaMethod, UserData, Value, Variadic}, typed::{
        Type, TypedDataFields, TypedDataMethods, TypedUserData,
        generator::{
            Definition, DefinitionFileGenerator, Definitions, LuauDefinitionFileGenerator,
        },
    }, typeduserdata_impl,
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
impl TypedUserData for SystemColor {}

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

    fn add_methods<T: TypedDataMethods<Self>>(methods: &mut T) {
        methods.add_meta_method(MetaMethod::ToString, |_lua, this, _: ()| {
            Ok(format!("{this:?}"))
        });
    }
}

impl FromLua for Color {
    fn from_lua(value: Value, lua: &Lua) -> mlua::prelude::LuaResult<Self> {
        match value {
            Value::UserData(data) => data.borrow::<Self>().map(|v| *v),
            // Use serde deserialize if not userdata
            other => lua.from_value(other),
        }
    }
}

/// This is a doc comment section for the overall type
#[derive(Debug, Clone, Copy, TypedUserData, Deserialize)]
struct Example {
    /// Example complex type
    color: Color,
}

#[typeduserdata_impl]
impl Example {
    /// print all items
    #[lua(name = "printAll", infallible)]
    fn print_all(all: Variadic<String>) {}

    /// Log a specific format with any lua types
    #[lua(name = "logAny", infallible)]
    fn log_any(format: String, inject: Variadic<String>) {}

    #[lua(meta, infallible)]
    fn __tostring(&self) -> String {
        format!("{self:?}")
    }
}

impl Default for Example {
    fn default() -> Self {
        Self {
            color: Color::Rgb(30, 132, 129),
        }
    }
}

impl FromLua for Example {
    fn from_lua(value: Value, lua: &Lua) -> mlua::prelude::LuaResult<Self> {
        match value {
            Value::UserData(data) => data.borrow::<Self>().map(|v| *v),
            other => lua.from_value(other),
        }
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

    let definitions: Definitions = Definitions::start()
        .define(
            "init",
            Definition::start()
                .register::<SystemColor>("System")
                .register::<Color>("Color")
                .register::<Example>("Example")
                .proxy::<Example>("example")
                .document("Greet someone")
                .param("name", "Name of the person to greet")
                .function::<String, ()>("greet", ())
                .param("color", "Color to print to stdout")
                .function::<Color, ()>("printColor", ()),
        )
        .finish();

    let types_path = PathBuf::from("examples/types");
    if !types_path.exists() {
        std::fs::create_dir_all(&types_path).unwrap();
    }

    let dfg = DefinitionFileGenerator::new(definitions.clone());
    for (name, writer) in dfg.iter() {
        println!("==== Generated \x1b[1;33mexample/types/{name}\x1b[0m ====");
        writer.write_file(types_path.join(name)).unwrap();
    }

    let luau_gen = LuauDefinitionFileGenerator::new(definitions);
    for (name, writer) in luau_gen.iter() {
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

    println!(
        "{:#?}",
        Type::string() | "literal" | true | 0 | [Type::string(), Type::nil(), Type::literal(3)]
    );

    Ok(())
}
