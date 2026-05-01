#![cfg(test)]

use crate::typed::{
    Index, Param, Type, Typed, TypedClassBuilder, TypedUserData,
    function::Return,
    generator::{Definition, DefinitionBuilder, DefinitionFileGenerator, Definitions, Entry},
};

/// Write definitions to a string buffer and return the output.
fn generate(definitions: Definitions) -> String {
    let dfg = DefinitionFileGenerator::new(definitions);
    let mut out = Vec::new();
    for (_, writer) in dfg.iter() {
        writer.write(&mut out).unwrap();
    }
    String::from_utf8(out).unwrap()
}

/// Build a single-file Definitions from a DefinitionBuilder.
fn single(def: impl Into<Definition>) -> Definitions {
    Definitions::start().define("test", def).finish()
}

/// Helper: add a typed value entry to a DefinitionBuilder.
fn with_value(
    mut builder: DefinitionBuilder,
    name: &str,
    ty: Type,
    doc: Option<&str>,
) -> DefinitionBuilder {
    builder
        .entries
        .push(Entry::new_with(name, Type::Value(Box::new(ty)), doc));
    builder
}

/// Resolve the lua-language-server binary path.
/// Checks `TEST_LUALS` env var first, then probes `lua-language-server` on PATH.
/// Returns `None` if not available or `TEST_LUALS=0`.
fn find_lua_ls() -> Option<String> {
    const LUA_LS: &str = "lua-language-server";

    if let Ok(path) = std::env::var("TEST_LUALS") {
        return (path != "0").then_some(path);
    }

    static PROBED: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| {
        std::process::Command::new(LUA_LS)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
    });
    PROBED.then(|| LUA_LS.to_string())
}

/// Write the generated definitions and a test script to a temp workspace,
/// run lua-language-server --check, and assert no diagnostics are produced.
/// Skips if lua-language-server is not available.
fn validate_with_lua_ls(defs_content: &str, script: &str) {
    let Some(lua_ls) = find_lua_ls() else { return };

    let dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let log_dir = tempfile::TempDir::new().expect("failed to create log dir");
    let meta_dir = tempfile::TempDir::new().expect("failed to create meta dir");

    std::fs::write(dir.path().join("defs.d.lua"), defs_content).unwrap();
    std::fs::write(dir.path().join("test.lua"), script).unwrap();
    std::fs::write(
        dir.path().join(".luarc.json"),
        r#"{"workspace.library": ["./"]}"#,
    )
    .unwrap();

    let output = std::process::Command::new(&lua_ls)
        .args([
            &format!("--check={}", dir.path().display()),
            "--check_format=json",
            "--checklevel=Warning",
            &format!("--logpath={}", log_dir.path().display()),
            &format!("--metapath={}", meta_dir.path().display()),
        ])
        .output()
        .unwrap_or_else(|e| panic!("failed to run lua-language-server ({lua_ls}): {e}"));

    assert!(
        output.status.success(),
        "lua-language-server reported diagnostics:\n{}\n\nGenerated definitions:\n{}",
        std::fs::read_to_string(log_dir.path().join("check.json")).unwrap_or_default(),
        defs_content,
    );
}

// ========================
// Output content tests
// ========================

#[test]
fn test_meta_header() {
    // Every output file starts with the @meta annotation required by LuaLS.
    let out = generate(single(Definition::start().function::<(), ()>("x", ())));
    assert_eq!(
        out.trim(),
        "--- @meta

function x() end"
    );
}

#[test]
fn test_extension_default() {
    let definitions = Definitions::start()
        .define("init", Definition::start().function::<(), ()>("x", ()))
        .finish();
    let dfg = DefinitionFileGenerator::new(definitions);
    let names: Vec<String> = dfg.iter().map(|(name, _)| name).collect();
    assert_eq!(names, vec!["init.d.lua"]);
}

#[test]
fn test_extension_custom() {
    let definitions = Definitions::start()
        .define("init", Definition::start().function::<(), ()>("x", ()))
        .finish();
    let dfg = DefinitionFileGenerator::new(definitions).ext(".lua");
    let names: Vec<String> = dfg.iter().map(|(name, _)| name).collect();
    assert_eq!(names, vec!["init.lua"]);
}

// --- Enum ---

#[test]
fn test_enum_single_variant() {
    let out = generate(single(
        Definition::start().register_as("Status", Type::r#enum([Type::literal("Active")])),
    ));
    assert_eq!(
        out.trim(),
        r#"--- @meta

--- @alias Status "Active""#
    );
}

#[test]
fn test_enum_multiple_variants() {
    let out = generate(single(Definition::start().register_as(
        "Direction",
        Type::r#enum([
            Type::literal("Up"),
            Type::literal("Down"),
            Type::literal("Left"),
            Type::literal("Right"),
        ]),
    )));
    assert_eq!(
        out.trim(),
        r#"--- @meta

--- @alias Direction "Up"
---| "Down"
---| "Left"
---| "Right""#
    );
}

#[test]
fn test_enum_with_doc() {
    let mut def = Definition::start();
    def.entries.push(Entry::new_with(
        "Color",
        Type::r#enum([Type::literal("Red"), Type::literal("Blue")]),
        Some("The available colors"),
    ));
    let out = generate(single(def));
    assert_eq!(
        out.trim(),
        r#"--- @meta

--- The available colors
--- @alias Color "Red"
---| "Blue""#
    );
}

// --- Alias ---

#[test]
fn test_alias_union() {
    let out = generate(single(
        Definition::start().register_as("StringOrNum", Type::string() | Type::number()),
    ));
    assert_eq!(
        out.trim(),
        "--- @meta

--- @alias StringOrNum string | number"
    );
}

#[test]
fn test_alias_named_type() {
    let out = generate(single(
        Definition::start().register_as("Name", Type::named("string")),
    ));
    assert_eq!(
        out.trim(),
        "--- @meta

--- @alias Name string"
    );
}

// --- Value ---

#[test]
fn test_value_primitive() {
    let out = generate(single(with_value(
        Definition::start(),
        "foo",
        Type::string(),
        None,
    )));
    assert_eq!(
        out.trim(),
        "--- @meta

--- @type string
foo = nil"
    );
}

#[test]
fn test_value_with_doc() {
    let out = generate(single(with_value(
        Definition::start(),
        "foo",
        Type::string(),
        Some("A global string"),
    )));
    assert_eq!(
        out.trim(),
        "--- @meta

--- A global string
--- @type string
foo = nil"
    );
}

#[test]
fn test_value_named_class_type() {
    // Use Type::named() to reference a class by name without going through name_map.
    let out = generate(single(with_value(
        Definition::start().register_as(
            "Player",
            Type::class(
                TypedClassBuilder::default()
                    .field("name", Type::string(), ())
                    .build(),
            ),
        ),
        "player",
        Type::named("Player"),
        None,
    )));
    assert_eq!(
        out.trim(),
        "--- @meta

--- @class Player
--- @field name string

--- @type Player
player = nil"
    );
}

// --- Root-level functions ---

#[test]
fn test_function_no_params_no_return() {
    let out = generate(single(Definition::start().function::<(), ()>("foo", ())));
    assert_eq!(
        out.trim(),
        "--- @meta

function foo() end"
    );
}

#[test]
fn test_function_named_params() {
    let out = generate(single(
        Definition::start()
            .param("name", "The name")
            .function::<String, ()>("greet", ()),
    ));
    assert_eq!(
        out.trim(),
        "--- @meta

--- @param name string The name
function greet(name) end"
    );
}

#[test]
fn test_function_unnamed_params() {
    let out = generate(single(
        Definition::start().function::<(String, i64), ()>("log", ()),
    ));
    assert_eq!(
        out.trim(),
        "--- @meta

--- @param param1 string
--- @param param2 integer
function log(param1, param2) end"
    );
}

#[test]
fn test_function_with_return() {
    let out = generate(single(
        Definition::start()
            .param("name", "")
            .function::<String, String>("greet", ()),
    ));
    assert_eq!(
        out.trim(),
        "--- @meta

--- @param name string
--- @return string
function greet(name) end"
    );
}

#[test]
fn test_function_multi_return() {
    let out = generate(single(
        Definition::start().function::<String, (bool, String)>("parse", ()),
    ));
    assert_eq!(
        out.trim(),
        "--- @meta

--- @param param1 string
--- @return boolean
--- @return string
function parse(param1) end"
    );
}

#[test]
fn test_function_with_doc() {
    let out = generate(single(
        Definition::start()
            .document("Greet someone\nThis is multiline")
            .function::<String, ()>("greet", ()),
    ));
    assert_eq!(
        out.trim(),
        "--- @meta

--- Greet someone
--- This is multiline
--- @param param1 string
function greet(param1) end"
    );
}

#[test]
fn test_function_escaped_name() {
    // Function names containing characters outside [a-zA-Z0-9_] are wrapped
    // in ["..."] by escape_key.
    let out = generate(single(
        Definition::start().function::<(), ()>("some.name", ()),
    ));
    assert_eq!(
        out.trim(),
        r#"--- @meta

function ["some.name"]() end"#
    );
}

// --- Class ---

#[test]
fn test_class_empty() {
    let out = generate(single(
        Definition::start().register_as("Empty", Type::class(TypedClassBuilder::default().build())),
    ));
    assert_eq!(
        out.trim(),
        "--- @meta

--- @class Empty"
    );
}

#[test]
fn test_class_with_fields() {
    let out = generate(single(
        Definition::start().register_as(
            "Player",
            Type::class(
                TypedClassBuilder::default()
                    .field("name", Type::string(), ())
                    .field("score", Type::integer(), ())
                    .build(),
            ),
        ),
    ));
    assert_eq!(
        out.trim(),
        "--- @meta

--- @class Player
--- @field name string
--- @field score integer"
    );
}

#[test]
fn test_class_field_with_doc() {
    let out = generate(single(
        Definition::start().register_as(
            "Player",
            Type::class(
                TypedClassBuilder::default()
                    .field("name", Type::string(), "The player's name")
                    .build(),
            ),
        ),
    ));
    assert_eq!(
        out.trim(),
        "--- @meta

--- @class Player
--- The player's name
--- @field name string"
    );
}

#[test]
fn test_class_doc_comments() {
    let mut builder = TypedClassBuilder::default();
    builder.typed_class.type_doc = Some("A class-level doc".into());
    let mut def_builder = Definition::start();
    def_builder.entries.push(Entry::new_with(
        "Documented",
        Type::class(builder.build()),
        Some("Top-level doc"),
    ));
    let out = generate(single(def_builder));
    assert_eq!(
        out.trim(),
        "--- @meta

--- Top-level doc
--- A class-level doc
--- @class Documented"
    );
}

#[test]
fn test_class_with_method_no_extra_params() {
    let out = generate(single(
        Definition::start().register_as(
            "Foo",
            Type::class(
                TypedClassBuilder::default()
                    .method::<(), String>("getValue", "Get the value")
                    .build(),
            ),
        ),
    ));
    assert_eq!(
        out.trim(),
        "--- @meta

--- @class Foo
local _CLASS_Foo_ = {
\t--- Get the value
\t--- @param self Foo
\t--- @return string
\tgetValue = function(self) end,
}"
    );
}

#[test]
fn test_class_with_method_with_params() {
    let out = generate(single(
        Definition::start().register_as(
            "Counter",
            Type::class(
                TypedClassBuilder::default()
                    .method::<(i64,), ()>("add", ())
                    .build(),
            ),
        ),
    ));
    assert_eq!(
        out.trim(),
        "--- @meta

--- @class Counter
local _CLASS_Counter_ = {
\t--- @param self Counter
\t--- @param param1 integer
\tadd = function(self, param1) end,
}"
    );
}

#[test]
fn test_class_with_static_function() {
    let out = generate(single(
        Definition::start().register_as(
            "Utils",
            Type::class(
                TypedClassBuilder::default()
                    .function::<String, i64>("create", "A factory")
                    .build(),
            ),
        ),
    ));
    assert_eq!(
        out.trim(),
        "--- @meta

--- @class Utils
local _CLASS_Utils_ = {
\t--- A factory
\t--- @param param1 string
\t--- @return integer
\tcreate = function(param1) end,
}"
    );
}

#[test]
fn test_class_fields_and_methods_combined() {
    let out = generate(single(
        Definition::start().register_as(
            "Player",
            Type::class(
                TypedClassBuilder::default()
                    .field("name", Type::string(), ())
                    .method::<(), String>("getName", ())
                    .build(),
            ),
        ),
    ));
    assert_eq!(
        out.trim(),
        "--- @meta

--- @class Player
--- @field name string
local _CLASS_Player_ = {
\t--- @param self Player
\t--- @return string
\tgetName = function(self) end,
}"
    );
}

#[test]
fn test_class_with_meta_field() {
    let out = generate(single(
        Definition::start().register_as(
            "Tracked",
            Type::class(
                TypedClassBuilder::default()
                    .meta_field("__count", Type::integer(), "Meta count")
                    .build(),
            ),
        ),
    ));
    assert_eq!(
        out.trim(),
        "--- @meta

--- @class Tracked
local _CLASS_Tracked_ = {
\t__metatable = {
\t\t--- Meta count
\t\t--- @type integer
\t\t__count = nil,
\t}
}"
    );
}

#[test]
fn test_class_with_meta_method() {
    let out = generate(single(
        Definition::start().register_as(
            "Obj",
            Type::class(
                TypedClassBuilder::default()
                    .field("x", Type::number(), ())
                    .meta_method::<(), String>("__tostring", ())
                    .build(),
            ),
        ),
    ));
    assert_eq!(
        out.trim(),
        "--- @meta

--- @class Obj
--- @field x number
local _CLASS_Obj_ = {
\t__metatable = {
\t\t--- @param self Obj
\t\t--- @return string
\t\t__tostring = function(self) end,
\t}
}"
    );
}

#[test]
fn test_class_with_meta_function() {
    let out = generate(single(
        Definition::start().register_as(
            "Indexed",
            Type::class(
                TypedClassBuilder::default()
                    .meta_function::<(String,), String>("__index", ())
                    .build(),
            ),
        ),
    ));
    assert_eq!(
        out.trim(),
        "--- @meta

--- @class Indexed
local _CLASS_Indexed_ = {
\t__metatable = {
\t\t--- @param param1 string
\t\t--- @return string
\t\t__index = function(param1) end,
\t}
}"
    );
}

// --- Inline type signatures ---

#[test]
fn test_type_sig_array() {
    let out = generate(single(
        Definition::start().register_as(
            "Names",
            Type::class(
                TypedClassBuilder::default()
                    .field("items", Type::array(Type::string()), ())
                    .build(),
            ),
        ),
    ));
    assert_eq!(
        out.trim(),
        "--- @meta

--- @class Names
--- @field items string[]"
    );
}

#[test]
fn test_type_sig_tuple() {
    let out = generate(single(
        Definition::start().register_as(
            "Pair",
            Type::class(
                TypedClassBuilder::default()
                    .field(
                        "coords",
                        Type::tuple([Type::integer(), Type::integer()]),
                        (),
                    )
                    .build(),
            ),
        ),
    ));
    assert_eq!(
        out.trim(),
        "--- @meta

--- @class Pair
--- @field coords [integer, integer]"
    );
}

#[test]
fn test_type_sig_map() {
    let out = generate(single(
        Definition::start().register_as(
            "Registry",
            Type::class(
                TypedClassBuilder::default()
                    .field("data", Type::map(Type::string(), Type::number()), ())
                    .build(),
            ),
        ),
    ));
    assert_eq!(
        out.trim(),
        "--- @meta

--- @class Registry
--- @field data { [string]: number }"
    );
}

#[test]
fn test_type_sig_table() {
    let out = generate(single(
        Definition::start().register_as(
            "Config",
            Type::class(
                TypedClassBuilder::default()
                    .field(
                        "opts",
                        Type::table([
                            (Index::from("host"), Type::string()),
                            (Index::from("port"), Type::integer()),
                        ]),
                        (),
                    )
                    .build(),
            ),
        ),
    ));
    assert_eq!(
        out.trim(),
        "--- @meta

--- @class Config
--- @field opts { host: string, port: integer }"
    );
}

#[test]
fn test_type_sig_union() {
    let out = generate(single(
        Definition::start().register_as(
            "Container",
            Type::class(
                TypedClassBuilder::default()
                    .field("value", Type::string() | Type::nil(), ())
                    .build(),
            ),
        ),
    ));
    // Note: the LuaLS generator does not have optional-type sugar (no `string?`);
    // `string | nil` is emitted as-is. This differs from the Luau generator.
    assert_eq!(
        out.trim(),
        "--- @meta

--- @class Container
--- @field value string | nil"
    );
}

#[test]
fn test_type_sig_function_inline() {
    let out = generate(single(
        Definition::start().register_as(
            "Handler",
            Type::class(
                TypedClassBuilder::default()
                    .field(
                        "callback",
                        Type::Function {
                            params: vec![Param {
                                name: Some("x".into()),
                                ty: Type::number(),
                                doc: None,
                            }],
                            returns: vec![Return {
                                ty: Type::boolean(),
                                doc: None,
                            }],
                        },
                        (),
                    )
                    .build(),
            ),
        ),
    ));
    assert_eq!(
        out.trim(),
        "--- @meta

--- @class Handler
--- @field callback fun(x: number): boolean"
    );
}

#[test]
fn test_type_sig_enum_cross_reference() {
    // Exercises the name_map: a field typed as a previously-registered enum
    // resolves to the enum's registered name rather than inlining its variants.
    let color_enum = Type::r#enum([Type::literal("Red"), Type::literal("Green")]);
    let out = generate(single(
        Definition::start()
            .register_as("Color", color_enum.clone())
            .register_as(
                "Widget",
                Type::class(
                    TypedClassBuilder::default()
                        .field("color", color_enum, ())
                        .build(),
                ),
            ),
    ));
    assert_eq!(
        out.trim(),
        r#"--- @meta

--- @alias Color "Red"
---| "Green"

--- @class Widget
--- @field color Color"#
    );
}

#[test]
fn test_type_sig_class_cross_reference() {
    // Exercises the name_map: a field typed as a previously-registered class
    // resolves to the class's registered name.
    let vec2 = Type::class(
        TypedClassBuilder::default()
            .field("x", Type::number(), ())
            .field("y", Type::number(), ())
            .build(),
    );
    let out = generate(single(
        Definition::start()
            .register_as("Vec2", vec2.clone())
            .register_as(
                "Sprite",
                Type::class(
                    TypedClassBuilder::default()
                        .field("position", vec2, ())
                        .build(),
                ),
            ),
    ));
    assert_eq!(
        out.trim(),
        "--- @meta

--- @class Vec2
--- @field x number
--- @field y number

--- @class Sprite
--- @field position Vec2"
    );
}

#[test]
fn test_enum_referenced_in_value() {
    let color_enum = Type::r#enum([
        Type::literal("Red"),
        Type::literal("Green"),
        Type::literal("Blue"),
    ]);
    let out = generate(single(with_value(
        Definition::start().register_as("Color", color_enum),
        "defaultColor",
        Type::named("Color"),
        None,
    )));
    assert_eq!(
        out.trim(),
        r#"--- @meta

--- @alias Color "Red"
---| "Green"
---| "Blue"

--- @type Color
defaultColor = nil"#
    );
}

/// Verify user data maps correctly
/// This tests a custom struct TestUserData, including attrs, functions and methods
/// It also test UserDataRef and UserDataRefMut in the function / method
#[test]
fn test_user_data() {
    struct TestUserData {
        attr: String,
    }
    impl mlua::UserData for TestUserData {}

    impl Typed for TestUserData {
        fn ty() -> Type {
            Type::class(TypedClassBuilder::new::<Self>().build())
        }

        fn as_param() -> Type {
            Type::named("TestUserData")
        }

        fn as_return() -> Type {
            Self::as_param()
        }
    }

    impl TypedUserData for TestUserData {
        fn add_documentation<F: crate::typed::TypedDataDocumentation<Self>>(docs: &mut F) {
            docs.add("class doc test");
        }

        fn add_fields<F: crate::typed::TypedDataFields<Self>>(fields: &mut F) {
            fields.document("attr doc test");
            fields.add_field_method_get("attr", |_, this| Ok(this.attr.clone()));
        }

        fn add_methods<T: crate::typed::TypedDataMethods<Self>>(methods: &mut T) {
            methods.document("function doc test");
            methods.add_function("from", |_, other: mlua::UserDataRef<Self>| {
                Ok(Self {
                    attr: other.attr.clone(),
                })
            });
            methods.document("method doc test");
            methods.add_method("to", |_, this, mut other: mlua::UserDataRefMut<Self>| {
                (*other).attr = this.attr.clone();
                Ok(())
            });
        }
    }

    let out = generate(single(
        Definition::start().register_as("TestUserData", TestUserData::ty()),
    ));
    assert_eq!(
        out.trim(),
        r#"--- @meta

--- class doc test
--- @class TestUserData
--- attr doc test
--- @field attr string
local _CLASS_TestUserData_ = {
	--- function doc test
	--- @param param1 TestUserData
	--- @return TestUserData
	from = function(param1) end,
	--- method doc test
	--- @param self TestUserData
	--- @param param1 TestUserData
	to = function(self, param1) end,
}"#
    );
}

// ========================
// lua-language-server validation tests
// ========================

#[test]
fn test_luals_function_valid_call() {
    let out = generate(single(
        Definition::start()
            .param("name", "")
            .function::<String, ()>("greet", ()),
    ));
    validate_with_lua_ls(&out, r#"greet("world")"#);
}

#[test]
fn test_luals_function_wrong_arg_type() {
    let out = generate(single(
        Definition::start()
            .param("name", "")
            .function::<String, ()>("greet", ()),
    ));

    let Some(lua_ls) = find_lua_ls() else { return };

    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = tempfile::TempDir::new().unwrap();
    let meta_dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("defs.d.lua"), &out).unwrap();
    std::fs::write(
        dir.path().join("test.lua"),
        "greet(42)
",
    )
    .unwrap();
    std::fs::write(
        dir.path().join(".luarc.json"),
        r#"{"workspace.library": ["./"]}"#,
    )
    .unwrap();

    let status = std::process::Command::new(&lua_ls)
        .args([
            &format!("--check={}", dir.path().display()),
            "--check_format=json",
            "--checklevel=Warning",
            &format!("--logpath={}", log_dir.path().display()),
            &format!("--metapath={}", meta_dir.path().display()),
        ])
        .status()
        .unwrap_or_else(|e| panic!("failed to run lua-language-server ({lua_ls}): {e}"));

    assert!(
        !status.success(),
        "expected lua-language-server to flag greet(42) as a type error"
    );
}

#[test]
fn test_luals_function_return_type() {
    let out = generate(single(
        Definition::start()
            .param("name", "")
            .function::<String, String>("greet", ()),
    ));
    validate_with_lua_ls(
        &out,
        r#"---@type string
local _result = greet("hello")
"#,
    );
}

#[test]
fn test_luals_class_field_access() {
    let out = generate(single(with_value(
        Definition::start().register_as(
            "Player",
            Type::class(
                TypedClassBuilder::default()
                    .field("name", Type::string(), ())
                    .field("score", Type::integer(), ())
                    .build(),
            ),
        ),
        "player",
        Type::named("Player"),
        None,
    )));
    validate_with_lua_ls(
        &out,
        "---@type string
local _name = player.name
---@type integer
local _score = player.score
",
    );
}

#[test]
fn test_luals_class_method_call() {
    let out = generate(single(with_value(
        Definition::start().register_as(
            "Player",
            Type::class(
                TypedClassBuilder::default()
                    .method::<(), String>("getName", ())
                    .build(),
            ),
        ),
        "player",
        Type::named("Player"),
        None,
    )));
    validate_with_lua_ls(
        &out,
        "---@type string
local _name = player:getName()
",
    );
}

#[test]
fn test_luals_enum_valid_assignment() {
    let out = generate(single(with_value(
        Definition::start().register_as(
            "Direction",
            Type::r#enum([Type::literal("Up"), Type::literal("Down")]),
        ),
        "dir",
        Type::named("Direction"),
        None,
    )));
    validate_with_lua_ls(
        &out,
        "local _d = dir
",
    );
}

#[test]
fn test_luals_enum_invalid_assignment() {
    let out = generate(single(with_value(
        Definition::start().register_as(
            "Direction",
            Type::r#enum([Type::literal("Up"), Type::literal("Down")]),
        ),
        "dir",
        Type::named("Direction"),
        None,
    )));

    let Some(lua_ls) = find_lua_ls() else { return };

    let dir_tmp = tempfile::TempDir::new().unwrap();
    let log_dir = tempfile::TempDir::new().unwrap();
    let meta_dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir_tmp.path().join("defs.d.lua"), &out).unwrap();
    // Assigning an integer to a Direction-typed variable should be a type mismatch.
    std::fs::write(
        dir_tmp.path().join("test.lua"),
        "---@type Direction
local _x = 42
",
    )
    .unwrap();
    std::fs::write(
        dir_tmp.path().join(".luarc.json"),
        r#"{"workspace.library": ["./"]}"#,
    )
    .unwrap();

    let status = std::process::Command::new(&lua_ls)
        .args([
            &format!("--check={}", dir_tmp.path().display()),
            "--check_format=json",
            "--checklevel=Warning",
            &format!("--logpath={}", log_dir.path().display()),
            &format!("--metapath={}", meta_dir.path().display()),
        ])
        .status()
        .unwrap_or_else(|e| panic!("failed to run lua-language-server ({lua_ls}): {e}"));

    assert!(
        !status.success(),
        "expected lua-language-server to flag integer assignment to Direction as a type error"
    );
}

#[test]
fn test_luals_enum_referenced_in_class_field() {
    // Validates the cross-reference path: a class field typed as a registered
    // enum resolves and type-checks correctly.
    let color_enum = Type::r#enum([Type::literal("Red"), Type::literal("Green")]);
    let out = generate(single(with_value(
        Definition::start()
            .register_as("Color", color_enum.clone())
            .register_as(
                "Widget",
                Type::class(
                    TypedClassBuilder::default()
                        .field("color", color_enum, ())
                        .build(),
                ),
            ),
        "widget",
        Type::named("Widget"),
        None,
    )));
    validate_with_lua_ls(
        &out,
        "---@type Color
local _c = widget.color
",
    );
}

#[test]
fn test_luals_readme_greet_function() {
    // Reproduces the greet function from the README example and validates it
    // with lua-language-server.
    let out = generate(
        Definitions::start()
            .define(
                "init",
                Definition::start()
                    .document("Greet the name that was passed in")
                    .param("name", "Name of the person to greet")
                    .function::<String, ()>("greet", ()),
            )
            .finish(),
    );
    validate_with_lua_ls(&out, r#"greet("world")"#);
}
