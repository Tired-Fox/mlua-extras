#![cfg(all(feature = "mlua", feature = "macros"))]

use mlua::{FromLua, MetaMethod, UserData, Value};
use mlua_extras::{
    Typed,
    mlua::Lua,
    typed::{
        TypedDataFields, TypedDataMethods, TypedUserData, WrappedBuilder, generator::Definition,
    },
};

#[derive(Default, Debug, Clone, Typed)]
struct TestOption {
    val: Option<String>,
}

impl FromLua for TestOption {
    fn from_lua(value: Value, _lua: &Lua) -> mlua::Result<Self> {
        let tn = value.type_name();
        match value {
            Value::UserData(usr_data) => {
                if usr_data.is::<TestOption>() {
                    return usr_data.take::<TestOption>();
                }
            }
            _ => (),
        }

        Err(mlua::Error::FromLuaConversionError {
            from: tn,
            to: "TestOption".to_string(),
            message: Some("failed to convert to userdata TestOption".into()),
        })
    }
}

impl UserData for TestOption {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        let mut wrapper = WrappedBuilder::new(fields);
        TypedUserData::add_fields(&mut wrapper);
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        let mut wrapper = WrappedBuilder::new(methods);
        TypedUserData::add_methods(&mut wrapper);
    }
}

impl TypedUserData for TestOption {
    fn add_fields<F: TypedDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("val", |_, this| Ok(this.val.clone()));
    }

    fn add_methods<T: TypedDataMethods<Self>>(methods: &mut T) {
        methods.add_function(
            "func_returns_option_self",
            |_, ()| -> mlua::Result<Option<Self>> { Ok(Some(Self::default())) },
        );
        methods.add_function(
            "func_takes_option_self",
            |_, s: Option<Self>| -> mlua::Result<Self> {
                Ok(Self {
                    val: s.and_then(|v| v.val.clone()),
                })
            },
        );

        #[cfg(feature = "userdata-wrappers")]
        methods.add_function(
            "func_returns_arc_self",
            |_, ()| -> mlua::Result<std::sync::Arc<Self>> { Ok(Default::default()) },
        );
        #[cfg(feature = "userdata-wrappers")]
        methods.add_function(
            "func_returns_arc_mutex_self",
            |_, ()| -> mlua::Result<std::sync::Arc<std::sync::Mutex<Self>>> {
                Ok(Default::default())
            },
        );

        #[cfg(feature = "userdata-wrappers")]
        methods.add_function(
            "func_returns_rc_refcell_self",
            |_, ()| -> mlua::Result<std::rc::Rc<Self>> { Ok(Default::default()) },
        );
        #[cfg(feature = "userdata-wrappers")]
        methods.add_function(
            "func_returns_rc_refcell_self",
            |_, ()| -> mlua::Result<std::rc::Rc<std::cell::RefCell<Self>>> {
                Ok(Default::default())
            },
        );

        methods.add_method("clone", |_, this, ()| Ok(this.clone()));
        methods.add_method(
            "method_returns_option_self",
            |_, _this, ()| -> mlua::Result<Option<Self>> { Ok(None) },
        );
        methods.add_method(
            "method_takes_option_self",
            |_, _this, v: Option<Self>| -> mlua::Result<Option<String>> {
                Ok(v.and_then(|v| v.val).map(|v| v.to_string()))
            },
        );

        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(match this.val.as_ref() {
                Some(val) => val.clone(),
                None => "nil".to_owned(),
            })
        });
    }
}

#[test]
fn test_recursive_types_in_methods() {
    let lua = Lua::new();

    // Using it works fine
    lua.globals().set("obj", TestOption::default()).unwrap();
    let val: String = lua.load("return tostring(obj)").eval().unwrap();
    assert_eq!(val, "nil");

    lua.globals().set("obj", TestOption::default()).unwrap();
    let val: String = lua
        .load("return tostring(obj.func_returns_option_self())")
        .eval()
        .unwrap();
    assert_eq!(val, "nil");

    lua.globals().set("obj", TestOption::default()).unwrap();
    let val: String = lua
        .load("return tostring(obj.func_takes_option_self(obj))")
        .eval()
        .unwrap();
    assert_eq!(val, "nil");

    lua.globals().set("obj", TestOption::default()).unwrap();
    let val: String = lua
        .load("return tostring(obj:method_returns_option_self())")
        .eval()
        .unwrap();
    assert_eq!(val, "nil");

    lua.globals().set("obj", TestOption::default()).unwrap();
    let val: String = lua
        .load("local s = obj:clone():method_takes_option_self(obj); return tostring(s)")
        .eval()
        .unwrap();
    assert_eq!(val, "nil");

    Definition::start()
        .register::<TestOption>("Example")
        .value::<TestOption>("obj")
        .finish();
}
