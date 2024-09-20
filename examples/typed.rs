use std::{io::stdout, path::PathBuf};

use mlua::{FromLua, Lua, MetaMethod, UserDataMethods, Value};
use mlua_extras::{
    typed::{
        generator::{Definitions, TypeFileGenerator},
        Type, TypedDataFields, TypedDataMethods, TypedFunction, TypedUserData,
    },
    LuaExtras, Typed, UserData,
};

#[derive(Default, Debug, Typed, UserData, Clone, Copy)]
enum Color {
    #[default]
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Cyan,
    Magenta,
    White,
    Xterm(u8),
    Rgb(u8, u8, u8),
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
    fn from_lua(value: Value<'lua>, _lua: &'lua Lua) -> mlua::prelude::LuaResult<Self> {
        match value {
            Value::String(str) => Ok(match str.to_str()?.to_ascii_lowercase().as_str() {
                "black" => Self::Black,
                "red" => Self::Red,
                "green" => Self::Green,
                "yellow" => Self::Yellow,
                "blue" => Self::Blue,
                "cyan" => Self::Cyan,
                "magenta" => Self::Magenta,
                "white" => Self::White,
                other => {
                    return Err(mlua::Error::FromLuaConversionError {
                        from: "string",
                        to: "Color",
                        message: Some(format!("unknown system color {other}")),
                    })
                }
            }),
            Value::Integer(v) => {
                if let 0..=255 = v {
                    Ok(Self::Xterm(v as u8))
                } else {
                    Err(mlua::Error::FromLuaConversionError {
                        from: "integer",
                        to: "Color",
                        message: Some("xterm colors must be between 0 and 255".into()),
                    })
                }
            }
            Value::Table(tbl) => {
                let values = tbl
                    .clone()
                    .pairs::<mlua::Integer, mlua::Integer>()
                    .map(|v| {
                        v.and_then(|v| {
                            if let 0..=255 = v.1 {
                                Ok(v.1 as u8)
                            } else {
                                Err(mlua::Error::FromLuaConversionError {
                                    from: "integer",
                                    to: "Color",
                                    message: Some("rgb colors must be between 0 and 255".into()),
                                })
                            }
                        })
                    })
                    .collect::<mlua::Result<Vec<_>>>()?;
                if values.len() != 3 {
                    Err(mlua::Error::FromLuaConversionError {
                        from: "table",
                        to: "Color",
                        message: Some("rgb tables must be an array of three integer values".into()),
                    })
                } else {
                    let mut values = values.into_iter();
                    Ok(Self::Rgb(
                        values.next().unwrap(),
                        values.next().unwrap(),
                        values.next().unwrap(),
                    ))
                }
            }
            other => Err(mlua::Error::FromLuaConversionError {
                from: other.type_name(),
                to: "Color",
                message: None,
            }),
        }
    }
}

#[derive(Default, Debug, UserData, Typed)]
struct Example {
    color: Color,
}

impl<'lua> FromLua<'lua> for Example {
    fn from_lua(_value: Value<'lua>, _lua: &'lua Lua) -> mlua::prelude::LuaResult<Self> {
        Ok(Example::default())
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
        methods
            .document("print the Example userdata")
            .add_method("print", |_lua, this, _: ()| {
                println!("{:?}", this.color);
                Ok(())
            });

        methods.add_meta_method(MetaMethod::ToString, |_lua, this, ()| { Ok(format!("{this:?}"))});
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

    println!();

    // Example of parsing `Example` as a class
    //if let Type::Class(name, gen) = Type::class::<Example>("Example") {
    //    print!("\x1b[38;2;128;128;128m");
    //    for doc in gen.type_doc.iter() {
    //        println!("--- {}", doc.split('\n').collect::<Vec<_>>().join("\n---"));
    //    }
    //    println!("--- \x1b[36m@class\x1b[33m {name}\x1b[33m\x1b[38;2;128;128;128m");
    //    for (name, field) in gen.fields.iter() {
    //        for doc in field.docs.iter() {
    //            println!("--- {}", doc.split('\n').collect::<Vec<_>>().join("\n---"));
    //        }
    //        println!("--- \x1b[36m@field\x1b[38;2;128;128;128m {name} \x1b[33m{}\x1b[38;2;128;128;128m", field.ty.as_ref());
    //    }
    //
    //    for (name, method) in gen.methods.iter() {
    //        for doc in method.docs.iter() {
    //            println!("--- {}", doc.split('\n').collect::<Vec<_>>().join("\n---"));
    //        }
    //        println!("--- \x1b[36m@field \x1b[38;2;128;128;128m{name} \x1b[35mfun\x1b[39m(\x1b[31mself\x1b[39m{})\x1b[38;2;128;128;128m",
    //            method.params
    //                .iter()
    //                .enumerate()
    //                .map(|(i, p)| p.name.as_ref().map(|v| v.to_string()).unwrap_or(format!("param{i}")))
    //                .collect::<Vec<_>>()
    //                .join(", ")
    //        );
    //    }
    //    print!("\x1b[0m");
    //}

    let definitions = Definitions::generate("init")
        .register::<Example>()
        .register_enum::<Color>()?
        .value_with::<Example, _>("example", ["Example module"])
        .alias_with("options", Type::literal_string("literal"), ["Options"])
        .function_with::<String, (), _>("hello", (), ["Say hello to someone"])
        .finish();

    let types_path = PathBuf::from("examples/types");
    if !types_path.exists() {
        std::fs::create_dir_all(&types_path).unwrap();
    }

    let gen = TypeFileGenerator::new(definitions);
    for (name, definition) in gen.iter() {
        println!("==== \x1b[1;33mexample/types/{name}\x1b[0m ====");
        definition.write_file(types_path.join(name)).unwrap();
    }

    Ok(())
}
