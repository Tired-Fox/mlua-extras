#![cfg(all(feature = "mlua", feature = "macros"))]

use mlua::{AnyUserData, FromLua};
use mlua_extras::{
    TypedUserData,
    mlua::{self, Lua, Value},
    typeduserdata_impl,
};

// Test 1: Feild attribute parsing

#[allow(dead_code)]
#[derive(Clone, TypedUserData)]
struct TestNamedFields {
    normal: String,
    #[lua(skip)]
    skipped: bool,
    #[lua(get)]
    readonly: i32,
    #[lua(set)]
    writeonly: f64,
    #[lua(name = "colour")]
    color: String,
}

#[test]
fn test_named_field_registration() {
    let lua = Lua::new();

    lua.globals()
        .set(
            "obj",
            TestNamedFields {
                normal: "test".into(),
                skipped: true,
                readonly: 4,
                writeonly: 3.14,
                color: "red".into(),
            },
        )
        .unwrap();

    // Read + Write field
    let val: String = lua.load("return obj.normal").eval().unwrap();
    assert_eq!(val, "test");
    lua.load("obj.normal = 'testing'").exec().unwrap();
    let val: String = lua.load("return obj.normal").eval().unwrap();
    assert_eq!(val, "testing");

    // Skip field: not accessible (throws error)
    let result = lua.load("return obj.skipped").eval::<Value>();
    println!("{result:?}");
    assert!(result.is_err());

    // Readonly field: Fails on write
    let val: i32 = lua.load("return obj.readonly").eval().unwrap();
    assert_eq!(val, 4);
    let result = lua.load("obj.readonly = 100").exec();
    assert!(result.is_err());

    // Writeonly field: Fails on read
    let result = lua.load("return obj.writeonly").eval::<Value>();
    assert!(result.is_err());
    let result = lua.load("obj.writeonly = 6.28").exec();
    assert!(result.is_ok());

    // Renamed field: accessible only via the rename value
    let val: String = lua.load("return obj.colour").eval().unwrap();
    assert_eq!(val, "red");
    lua.load("obj.colour = 'blue'").exec().unwrap();
    let val: String = lua.load("return obj.colour").eval().unwrap();
    assert_eq!(val, "blue");

    let result = lua.load("return obj.color").eval::<Value>();
    assert!(result.is_err());
}

// Test 1: Methods with rename

#[derive(Clone, TypedUserData)]
struct Calculator {
    value: f64,
}

#[typeduserdata_impl]
impl Calculator {
    #[lua(infallible)]
    fn add(&self, x: f64) -> f64 {
        self.value + x
    }

    #[lua(name = "divide")]
    fn checked_divide(&self, x: f64) -> mlua::Result<f64> {
        if x == 0.0 {
            Err(mlua::Error::runtime("division by zero"))
        } else {
            Ok(self.value / x)
        }
    }

    #[lua(infallible)]
    fn get_value_and_double(&self) -> (f64, f64) {
        (self.value, self.value * 2.0)
    }
}

#[test]
fn test_method_registration() {
    let lua = Lua::new();

    lua.globals()
        .set("calc", Calculator { value: 10.0 })
        .unwrap();

    let val: f64 = lua.load("return calc.value").eval().unwrap();
    assert_eq!(val, 10.0);

    // Infallible
    let result: f64 = lua.load("return calc:add(5)").eval().unwrap();
    assert_eq!(result, 15.0);

    // Rename (fallible)
    let result: f64 = lua.load("return calc:divide(2)").eval().unwrap();
    assert_eq!(result, 5.0);

    // Fallible
    let result = lua.load("return calc:divide(0)").exec();
    assert!(result.is_err());

    // Multi-return method
    let (a, b): (f64, f64) = lua
        .load("return calc:get_value_and_double()")
        .eval()
        .unwrap();
    assert_eq!(a, 10.0);
    assert_eq!(b, 20.0);
}

// Test 2: Metamethods

#[derive(Clone, TypedUserData)]
struct Stringable {
    value: String,
}

#[typeduserdata_impl]
impl Stringable {
    #[lua(meta, infallible)]
    fn __tostring(&self) -> String {
        format!("Stringable({})", self.value)
    }

    #[lua(meta, infallible)]
    fn __len(&self) -> usize {
        self.value.len()
    }

    #[lua(meta, name = "__half", infallible)]
    fn first_half(&self) -> String {
        let c = self.__len();
        self.value[0..c / 2].to_string()
    }
}

#[test]
fn test_metamethods() {
    let lua = Lua::new();
    lua.globals()
        .set(
            "obj",
            Stringable {
                value: "hello, world!".into(),
            },
        )
        .unwrap();
    lua.globals()
        .set(
            "half",
            lua.create_function(|_lua, this: AnyUserData| {
                let metatable = this.metatable()?;
                if let Ok(half) = metatable.get::<mlua::Function>("__half") {
                    return half.call::<String>(this);
                }
                Err(mlua::Error::runtime(
                    "type does not implememnt __half metamethod",
                ))
            })
            .unwrap(),
        )
        .unwrap();

    let result: String = lua.load("return tostring(obj)").eval().unwrap();
    assert_eq!(result, "Stringable(hello, world!)");

    let result: i64 = lua.load("return #obj").eval().unwrap();
    assert_eq!(result, 13);

    let result: String = lua.load("return half(obj)").eval().unwrap();
    assert_eq!(result, "hello,");
}

// Test 3: Mutable Methods

#[derive(Clone, TypedUserData)]
struct MutCalc {
    value: f64,
}

#[typeduserdata_impl]
impl MutCalc {
    #[lua(infallible)]
    fn set_value(&mut self, x: f64) {
        self.value = x;
    }
}

#[test]
fn test_mut_method() {
    let lua = Lua::new();
    lua.globals().set("calc", MutCalc { value: 0.0 }).unwrap();

    lua.load("calc:set_value(42)").exec().unwrap();
    let result: f64 = lua.load("return calc.value").eval().unwrap();
    assert_eq!(result, 42.0);
}

// Test 4: Optional lua parameter

#[derive(Clone, TypedUserData)]
struct LuaAccess;

#[typeduserdata_impl]
impl LuaAccess {
    fn create_table(&self, lua: &Lua) -> mlua::Result<mlua::Table> {
        lua.create_table()
    }

    #[lua(infallible)]
    fn no_lua(&self) -> String {
        "test".into()
    }
}

#[test]
fn test_optional_lua_param() {
    let lua = Lua::new();
    lua.globals().set("obj", LuaAccess).unwrap();
    let result: mlua::Table = lua.load("return obj:create_table()").eval().unwrap();
    assert!(result.is_empty());
    let result: String = lua.load("return obj:no_lua()").eval().unwrap();
    assert_eq!(result, "test");
}

// Test 5: Static functions (no self)

#[derive(Clone, TypedUserData)]
struct MathUtils;

#[typeduserdata_impl]
impl MathUtils {
    #[lua(infallible)]
    fn add(a: f64, b: f64) -> f64 {
        a + b
    }

    #[lua(name = "create")]
    fn new_instance(lua: &Lua) -> mlua::Result<mlua::Table> {
        lua.create_table()
    }
}

#[test]
fn test_static_functions() {
    let lua = Lua::new();
    lua.globals().set("math", MathUtils).unwrap();

    let result: f64 = lua.load("return math.add(3, 4)").eval().unwrap();
    assert_eq!(result, 7.0);

    let result: mlua::Table = lua.load("return math.create()").eval().unwrap();
    assert!(result.is_empty());
}

// Test 6: Static meta functions (no self)

#[derive(Debug, Clone, TypedUserData, PartialEq)]
struct Vec2(f64, f64);
impl FromLua for Vec2 {
    fn from_lua(value: Value, _lua: &Lua) -> mlua::Result<Self> {
        let tn = value.type_name();
        match value {
            Value::UserData(usr_data) => {
                if usr_data.is::<Vec2>() {
                    return usr_data.take::<Vec2>();
                }
            }
            Value::Table(tbl) => {
                return Ok(Vec2(tbl.get(1)?, tbl.get(2)?));
            }
            Value::Number(n) => return Ok(Vec2(n, n)),
            _ => (),
        }

        Err(mlua::Error::FromLuaConversionError {
            from: tn,
            to: "Vec2".to_string(),
            message: Some("failed to convert to userdata Vec2".into()),
        })
    }
}

#[typeduserdata_impl]
impl Vec2 {
    #[lua(meta, infallible)]
    fn __add(a: Vec2, b: Vec2) -> Self {
        Vec2(a.0 + b.0, a.1 + b.1)
    }

    #[lua(meta, name = "__dot", infallible)]
    fn dot_product(a: Vec2, b: Vec2) -> f64 {
        (a.0 * b.0) + (a.1 * b.1)
    }
}

#[test]
fn test_static_meta_functions() {
    let lua = Lua::new();
    lua.globals()
        .set(
            "vec2",
            lua.create_function(|_lua, (x, y): (f64, f64)| Ok(Vec2(x, y)))
                .unwrap(),
        )
        .unwrap();
    lua.globals()
        .set(
            "dot",
            lua.create_function(|_lua, (a, b): (AnyUserData, AnyUserData)| {
                if a.type_id() != b.type_id() {
                    return Err(mlua::Error::runtime("both parameters but be the same type"));
                }

                let am = a.metatable()?;
                am.get::<mlua::Function>("__dot")?.call::<f64>((a, b))
            })
            .unwrap(),
        )
        .unwrap();

    let result: Vec2 = lua.load("return vec2(1, 2) + vec2(3, 4)").eval().unwrap();
    assert_eq!(result, Vec2(4.0, 6.0));

    let result: f64 = lua
        .load("return dot(vec2(2, 4), vec2(4, 2))")
        .eval()
        .unwrap();
    assert_eq!(result, 16.0);
}

// Test 7: Async Methods

#[cfg(feature = "async")]
mod async_tests {
    use super::*;

    #[derive(Clone, TypedUserData)]
    struct AsyncWorker {
        prefix: String,
    }

    #[typeduserdata_impl]
    impl AsyncWorker {
        async fn process(&self, input: String) -> mlua::Result<String> {
            Ok(format!("{}: {input}", self.prefix))
        }

        async fn with_lua(&self, lua: Lua, key: String) -> mlua::Result<Value> {
            lua.globals().get(key)
        }
    }

    #[tokio::test]
    async fn test_async_methods() {
        let lua = Lua::new();
        lua.globals()
            .set(
                "worker",
                AsyncWorker {
                    prefix: "test".into(),
                },
            )
            .unwrap();

        let result: String = lua
            .load("return worker:process('hello, world')")
            .eval_async()
            .await
            .unwrap();
        assert_eq!(result, "test: hello, world");

        lua.globals().set("test_val", 42).unwrap();
        let result: i64 = lua
            .load("return worker:with_lua('test_val')")
            .eval_async()
            .await
            .unwrap();
        assert_eq!(result, 42)
    }
}
