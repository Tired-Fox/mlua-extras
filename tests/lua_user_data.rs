#![cfg(all(feature = "mlua", feature = "derive"))]

use mlua_extras::{
    mlua::{self, Lua, Value},
    TypedUserData,
};

// ============================================================
// Test 1: Field attribute parsing
// ============================================================

#[derive(Clone, TypedUserData)]
struct TestFields {
    normal: String,
    #[mlua_extras(skip)]
    #[allow(unused)]
    skipped: bool,
    #[mlua_extras(readonly)]
    read_only: i32,
    #[mlua_extras(writeonly)]
    write_only: f64,
    #[mlua_extras(rename = "colour")]
    color: String,
}

#[test]
fn test_field_registration() {
    let lua = Lua::new();
    lua.globals()
        .set(
            "obj",
            TestFields {
                normal: "hello".into(),
                skipped: true,
                read_only: 42,
                write_only: 3.14,
                color: "red".into(),
            },
        )
        .unwrap();

    // ReadWrite field: read and write
    let val: String = lua.load("return obj.normal").eval().unwrap();
    assert_eq!(val, "hello");
    lua.load("obj.normal = 'world'").exec().unwrap();
    let val: String = lua.load("return obj.normal").eval().unwrap();
    assert_eq!(val, "world");

    // Skip field: not accessible (errors on access since UserData has no such field)
    let result = lua.load("return obj.skipped").eval::<Value>();
    assert!(result.is_err());

    // ReadOnly field: can read, cannot write
    let val: i32 = lua.load("return obj.read_only").eval().unwrap();
    assert_eq!(val, 42);
    let result = lua.load("obj.read_only = 100").exec();
    assert!(result.is_err());

    // WriteOnly field: can write
    lua.load("obj.write_only = 2.71").exec().unwrap();

    // Renamed field: accessible via Lua name
    let val: String = lua.load("return obj.colour").eval().unwrap();
    assert_eq!(val, "red");
    lua.load("obj.colour = 'blue'").exec().unwrap();
    let val: String = lua.load("return obj.colour").eval().unwrap();
    assert_eq!(val, "blue");
}

// ============================================================
// Test 2: Fields only (no methods block)
// ============================================================

#[derive(Clone, TypedUserData)]
struct Point {
    x: f64,
    y: f64,
}

#[test]
fn test_fields_only() {
    let lua = Lua::new();
    lua.globals()
        .set("p", Point { x: 1.0, y: 2.0 })
        .unwrap();

    let x: f64 = lua.load("return p.x").eval().unwrap();
    assert_eq!(x, 1.0);
    let y: f64 = lua.load("return p.y").eval().unwrap();
    assert_eq!(y, 2.0);

    lua.load("p.x = 10.0").exec().unwrap();
    let x: f64 = lua.load("return p.x").eval().unwrap();
    assert_eq!(x, 10.0);
}

// ============================================================
// Test 3: Methods with rename
// ============================================================

#[derive(Clone, TypedUserData)]
struct Calculator {
    value: f64,
}

#[mlua_extras::typed_user_data_impl]
impl Calculator {
    #[method]
    fn add(&self, x: f64) -> f64 {
        self.value + x
    }

    #[method(rename = "divide")]
    fn checked_divide(&self, x: f64) -> mlua::Result<f64> {
        if x == 0.0 {
            Err(mlua::Error::runtime("division by zero"))
        } else {
            Ok(self.value / x)
        }
    }

    #[method]
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

    // Read field via auto-generated getter
    let val: f64 = lua.load("return calc.value").eval().unwrap();
    assert_eq!(val, 10.0);

    // Infallible method
    let result: f64 = lua.load("return calc:add(5)").eval().unwrap();
    assert_eq!(result, 15.0);

    // Renamed method (fallible)
    let result: f64 = lua.load("return calc:divide(2)").eval().unwrap();
    assert_eq!(result, 5.0);

    // Fallible method - error
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

// ============================================================
// Test 4: Metamethods
// ============================================================

#[derive(Clone, TypedUserData)]
struct Stringable {
    value: String,
}

#[mlua_extras::typed_user_data_impl]
impl Stringable {
    #[metamethod(ToString)]
    fn to_string_repr(&self) -> String {
        format!("Stringable({})", self.value)
    }

    #[metamethod(Len)]
    fn len(&self) -> usize {
        self.value.len()
    }
}

#[test]
fn test_metamethods() {
    let lua = Lua::new();
    lua.globals()
        .set(
            "obj",
            Stringable {
                value: "hello".into(),
            },
        )
        .unwrap();

    let result: String = lua.load("return tostring(obj)").eval().unwrap();
    assert_eq!(result, "Stringable(hello)");

    let result: i64 = lua.load("return #obj").eval().unwrap();
    assert_eq!(result, 5);
}

// ============================================================
// Test 5: Mutable methods
// ============================================================

#[derive(Clone, TypedUserData)]
struct MutCalc {
    value: f64,
}

#[mlua_extras::typed_user_data_impl]
impl MutCalc {
    #[method]
    fn set_value(&mut self, x: f64) {
        self.value = x;
    }

    #[method]
    fn get_value(&self) -> f64 {
        self.value
    }
}

#[test]
fn test_mut_method() {
    let lua = Lua::new();
    lua.globals()
        .set("calc", MutCalc { value: 0.0 })
        .unwrap();
    lua.load("calc:set_value(42)").exec().unwrap();
    let result: f64 = lua.load("return calc:get_value()").eval().unwrap();
    assert_eq!(result, 42.0);
}

// ============================================================
// Test 6: Optional lua parameter
// ============================================================

#[derive(Clone, TypedUserData)]
struct LuaAccess;

#[mlua_extras::typed_user_data_impl]
impl LuaAccess {
    #[method]
    fn create_table(&self, lua: &Lua) -> mlua::Result<mlua::Table> {
        lua.create_table()
    }

    #[method]
    fn no_lua(&self) -> String {
        "hello".into()
    }
}

#[test]
fn test_optional_lua_param() {
    let lua = Lua::new();
    lua.globals().set("obj", LuaAccess).unwrap();
    let result: mlua::Table = lua.load("return obj:create_table()").eval().unwrap();
    assert!(result.is_empty());
    let result: String = lua.load("return obj:no_lua()").eval().unwrap();
    assert_eq!(result, "hello");
}

// ============================================================
// Test 7: Static functions (no self)
// ============================================================

#[derive(Clone, TypedUserData)]
struct MathUtils;

#[mlua_extras::typed_user_data_impl]
impl MathUtils {
    #[method]
    fn add(a: f64, b: f64) -> f64 {
        a + b
    }

    #[method(rename = "create")]
    fn new_instance(lua: &Lua) -> mlua::Result<mlua::Table> {
        lua.create_table()
    }
}

#[test]
fn test_static_function() {
    let lua = Lua::new();
    lua.globals().set("math_utils", MathUtils).unwrap();
    let result: f64 = lua.load("return math_utils.add(3, 4)").eval().unwrap();
    assert_eq!(result, 7.0);

    // Static function with lua param + rename
    let result: mlua::Table = lua.load("return math_utils.create()").eval().unwrap();
    assert!(result.is_empty());
}

// ============================================================
// Test 8: Full integration (fields + methods + metamethods + rename)
// ============================================================

#[derive(Clone, TypedUserData)]
struct Person {
    /// Person's name
    name: String,
    age: u8,
    #[mlua_extras(skip)]
    #[allow(unused)]
    internal_id: u64,
    #[mlua_extras(readonly)]
    created_at: String,
    #[mlua_extras(rename = "family_name")]
    last_name: String,
}

#[mlua_extras::typed_user_data_impl]
impl Person {
    #[method]
    fn get_name_and_age(&self) -> (String, u8) {
        (self.name.clone(), self.age)
    }

    #[method]
    fn greet(&self, lua: &Lua, greeting: String) -> mlua::Result<mlua::String> {
        lua.create_string(format!("{greeting}, {}!", self.name))
    }

    #[metamethod(ToString)]
    fn to_string(&self) -> String {
        format!("{} (age {})", self.name, self.age)
    }

    #[method(rename = "fullName")]
    fn full_name(&self) -> String {
        format!("{} {}", self.name, self.last_name)
    }
}

#[test]
fn test_full_integration() {
    let lua = Lua::new();
    lua.globals()
        .set(
            "person",
            Person {
                name: "Alice".into(),
                age: 30,
                internal_id: 12345,
                created_at: "2024-01-01".into(),
                last_name: "Smith".into(),
            },
        )
        .unwrap();

    // Fields
    let name: String = lua.load("return person.name").eval().unwrap();
    assert_eq!(name, "Alice");

    // Renamed field
    let family: String = lua.load("return person.family_name").eval().unwrap();
    assert_eq!(family, "Smith");

    // ReadOnly field
    let created: String = lua.load("return person.created_at").eval().unwrap();
    assert_eq!(created, "2024-01-01");

    // Skip field: not accessible
    let result: Value = lua.load("return person.internal_id").eval().unwrap();
    assert!(matches!(result, Value::Nil));

    // Methods
    let (name, age): (String, u8) = lua
        .load("return person:get_name_and_age()")
        .eval()
        .unwrap();
    assert_eq!(name, "Alice");
    assert_eq!(age, 30);

    // Method with lua param
    let greeting: String = lua
        .load(r#"return person:greet("Hello")"#)
        .eval()
        .unwrap();
    assert_eq!(greeting, "Hello, Alice!");

    // Metamethod
    let s: String = lua.load("return tostring(person)").eval().unwrap();
    assert_eq!(s, "Alice (age 30)");

    // Renamed method
    let full: String = lua.load("return person:fullName()").eval().unwrap();
    assert_eq!(full, "Alice Smith");
}

// ============================================================
// Test 9: Unit struct (no fields)
// ============================================================

#[derive(Clone, TypedUserData)]
struct UnitType;

#[mlua_extras::typed_user_data_impl]
impl UnitType {
    #[method]
    fn hello(&self) -> String {
        "hello from unit".into()
    }
}

#[test]
fn test_unit_struct() {
    let lua = Lua::new();
    lua.globals().set("u", UnitType).unwrap();
    let result: String = lua.load("return u:hello()").eval().unwrap();
    assert_eq!(result, "hello from unit");
}

// ============================================================
// Test 10: Async methods (requires async feature)
// ============================================================

#[cfg(feature = "async")]
mod async_tests {
    use super::*;

    #[derive(Clone, TypedUserData)]
    struct AsyncWorker {
        prefix: String,
    }

    #[mlua_extras::typed_user_data_impl]
    impl AsyncWorker {
        #[method]
        async fn process(&self, input: String) -> mlua::Result<String> {
            Ok(format!("{}: {}", self.prefix, input))
        }

        #[method]
        async fn with_lua(&self, lua: mlua::Lua, key: String) -> mlua::Result<Value> {
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
                    prefix: "hello".into(),
                },
            )
            .unwrap();

        let result: String = lua
            .load("return worker:process('world')")
            .eval_async()
            .await
            .unwrap();
        assert_eq!(result, "hello: world");

        lua.globals().set("test_val", 42).unwrap();
        let result: i64 = lua
            .load("return worker:with_lua('test_val')")
            .eval_async()
            .await
            .unwrap();
        assert_eq!(result, 42);
    }
}
