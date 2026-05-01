#![cfg(test)]
#![cfg(feature = "luau")]

use crate::typed::{
    Field, Func, Index, Param, Type, Typed, TypedClassBuilder, TypedUserData,
    function::Return,
    generator::{Definition, DefinitionBuilder, Definitions, Entry, LuauDefinitionFileGenerator},
};

/// Write definitions to a string buffer and return the output.
fn generate(definitions: Definitions) -> String {
    let ldfg = LuauDefinitionFileGenerator::new(definitions);
    let mut out = Vec::new();
    for (_, writer) in ldfg.iter() {
        writer.write(&mut out).unwrap();
    }
    String::from_utf8(out).unwrap()
}

/// Build a single-file Definitions from a DefinitionBuilder.
fn single(def: DefinitionBuilder) -> Definitions {
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

/// Resolve the luau-lsp binary path. If `TEST_LUAU` is set, use its value
/// as the binary path; otherwise fall back to `"luau-lsp"`. Then check
/// whether the binary exists on `PATH`. Returns `Some(path)` if found,
/// `None` if the binary is not available.
fn find_luau_lsp() -> Option<String> {
    const LUAU_LSP: &str = "luau-lsp";

    if let Ok(path) = std::env::var("TEST_LUAU") {
        return (path != "0").then_some(path);
    }

    // No env var set; probe for luau-lsp on PATH once
    static PROBED: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| {
        std::process::Command::new(LUAU_LSP)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
    });
    PROBED.then(|| LUAU_LSP.to_string())
}

/// If luau-lsp is available, write the output to a temp file and validate it
/// with `luau-lsp analyze`. Skips validation when the binary is not found.
fn validate_with_luau_lsp(defs_content: &str, script: &str) {
    let Some(luau_lsp) = find_luau_lsp() else {
        return;
    };

    let dir = tempfile::TempDir::new().expect("failed to create temp dir");

    let defs_path = dir.path().join("defs.d.luau");
    std::fs::write(&defs_path, defs_content).unwrap();

    let script_path = dir.path().join("test.luau");
    std::fs::write(&script_path, script).unwrap();

    let output = std::process::Command::new(&luau_lsp)
        .arg("analyze")
        .arg(format!("--defs=@test={}", defs_path.display()))
        .arg(&script_path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run luau-lsp ({luau_lsp}): {e}"));

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Filter out INFO lines — only keep actual errors
    let errors: Vec<&str> = stderr
        .lines()
        .filter(|l| !l.starts_with("[INFO]"))
        .collect();

    assert!(
        errors.is_empty(),
        "luau-lsp reported errors:\n{}\n\nGenerated definitions:\n{}",
        errors.join("\n"),
        defs_content,
    );
}

// ========================
// Output content tests
// ========================

#[test]
fn test_enum_type() {
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
        r#"export type Direction = "Up" | "Down" | "Left" | "Right""#
    );
}

#[test]
fn test_alias_type() {
    let out = generate(single(
        Definition::start().register_as("StringOrNum", Type::string() | Type::number()),
    ));
    assert_eq!(out.trim(), "export type StringOrNum = string | number");
}

#[test]
fn test_declare_value() {
    let out = generate(single(with_value(
        Definition::start(),
        "myGlobal",
        Type::string(),
        Some("A global"),
    )));
    assert_eq!(
        out.trim(),
        "-- A global
declare myGlobal: string"
    );
}

#[test]
fn test_declare_function() {
    let out = generate(single(
        Definition::start()
            .param("name", "The name")
            .ret("Formatted greeting")
            .function::<String, String>("greet", ()),
    ));
    assert_eq!(
        out.trim(),
        "-- @param name string -- The name
-- @return string -- #1 Formatted greeting
declare function greet(name: string): string"
    );
}

#[test]
fn test_function_no_return() {
    let out = generate(single(
        Definition::start().function::<(String,), ()>("log", ()),
    ));
    assert_eq!(out.trim(), "declare function log(param1: string): ()");
}

#[test]
fn test_function_multi_return() {
    let out = generate(single(
        Definition::start().function::<String, (bool, String)>("parse", ()),
    ));
    assert_eq!(
        out.trim(),
        "declare function parse(param1: string): (boolean, string)"
    );
}

#[test]
fn test_class_with_fields() {
    let out = generate(single(
        Definition::start().register_as(
            "Player",
            Type::class(
                TypedClassBuilder::default()
                    .field("name", Type::string(), "Player name")
                    .field("score", Type::integer(), ())
                    .build(),
            ),
        ),
    ));
    assert_eq!(
        out.trim(),
        "declare class Player
\t-- Player name
\tname: string
\tscore: number
end"
    );
}

#[test]
fn test_class_with_methods() {
    let out = generate(single(
        Definition::start().register_as(
            "Counter",
            Type::class(
                TypedClassBuilder::default()
                    .field("value", Type::integer(), ())
                    .method::<(), i64>("getValue", "Get the current value")
                    .method::<(i64,), ()>("add", ())
                    .build(),
            ),
        ),
    ));
    assert_eq!(
        out.trim(),
        "declare class Counter
\tvalue: number
\tfunction add(self, param1: number): ()
\t-- Get the current value
\tfunction getValue(self): number
end"
    );
}

#[test]
fn test_class_with_functions_separate_table() {
    let out = generate(single(
        Definition::start().register_as(
            "Utils",
            Type::class(
                TypedClassBuilder::default()
                    .function::<String, String>("upper", ())
                    .build(),
            ),
        ),
    ));
    // Static functions are emitted as a separate global table declaration
    assert_eq!(
        out.trim(),
        "declare class Utils
end

declare function Utils_upper(param1: string): string

declare Utils: {
\tupper: typeof(Utils_upper),
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
        "declare class Obj
\tx: number
\tfunction __tostring(self): string
end"
    );
}

#[test]
fn test_class_with_meta_field() {
    let mut builder = TypedClassBuilder::default();
    builder.typed_class.meta_fields.insert(
        Index::from("__count"),
        Field::new(Type::integer(), "Meta field"),
    );
    let out = generate(single(
        Definition::start().register_as("Tracked", Type::class(builder.build())),
    ));
    assert_eq!(
        out.trim(),
        "declare class Tracked
\t-- Meta field
\t__count: number
end"
    );
}

#[test]
fn test_optional_type_sugar() {
    let out = generate(single(
        Definition::start().register_as("MaybeStr", Type::string() | Type::nil()),
    ));
    assert_eq!(out.trim(), "export type MaybeStr = string?");
}

#[test]
fn test_array_type() {
    let out = generate(single(
        Definition::start().register_as("Names", Type::array(Type::string())),
    ));
    assert_eq!(out.trim(), "export type Names = { string }");
}

#[test]
fn test_map_type() {
    let out = generate(single(
        Definition::start().register_as("Scores", Type::map(Type::string(), Type::integer())),
    ));
    assert_eq!(out.trim(), "export type Scores = { [string]: number }");
}

#[test]
fn test_table_type() {
    let out = generate(single(Definition::start().register_as(
        "Config",
        Type::table([
            (Index::from("host"), Type::string()),
            (Index::from("port"), Type::integer()),
        ]),
    )));
    assert_eq!(
        out.trim(),
        "export type Config = { host: string, port: number }"
    );
}

#[test]
fn test_function_type_signature() {
    let func_type = Type::Function {
        params: vec![Param {
            name: Some("x".into()),
            ty: Type::number(),
            doc: None,
        }],
        returns: vec![Return {
            ty: Type::boolean(),
            doc: None,
        }],
    };
    let out = generate(single(
        Definition::start().register_as("Predicate", func_type),
    ));
    assert_eq!(out.trim(), "export type Predicate = (x: number) -> boolean");
}

#[test]
fn test_tuple_homogeneous() {
    let out = generate(single(
        Definition::start().register_as("Pair", Type::tuple([Type::integer(), Type::integer()])),
    ));
    assert_eq!(out.trim(), "export type Pair = { number }");
}

#[test]
fn test_tuple_heterogeneous() {
    let out = generate(single(Definition::start().register_as(
        "Mixed",
        Type::tuple([Type::string(), Type::integer(), Type::boolean()]),
    )));
    assert_eq!(
        out.trim(),
        "export type Mixed = { string | number | boolean }"
    );
}

#[test]
fn test_union_type() {
    let out = generate(single(
        Definition::start()
            .register_as("Multi", Type::string() | Type::integer() | Type::boolean()),
    ));
    assert_eq!(out.trim(), "export type Multi = string | number | boolean");
}

#[test]
fn test_doc_comments() {
    let out = generate(single(
        Definition::start()
            .document("Greet someone\nThis is multiline")
            .function::<String, ()>("greet", ()),
    ));
    assert_eq!(
        out.trim(),
        "-- Greet someone
-- This is multiline
declare function greet(param1: string): ()"
    );
}

#[test]
fn test_class_doc_comment() {
    let mut builder = TypedClassBuilder::default();
    builder.typed_class.type_doc = Some("A documented class".into());
    // register_as uses Entry::new (no doc), so set doc on the entry directly
    let mut def_builder = Definition::start();
    def_builder.entries.push(Entry::new_with(
        "Documented",
        Type::class(builder.build()),
        Some("Top-level doc"),
    ));
    let out = generate(single(def_builder));
    assert_eq!(
        out.trim(),
        "-- Top-level doc
-- A documented class
declare class Documented
end"
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
        "export type Color = \"Red\" | \"Green\" | \"Blue\"

declare defaultColor: Color"
    );
}

#[test]
fn test_extension_default() {
    let definitions = Definitions::start()
        .define("init", Definition::start().register_as("X", Type::string()))
        .finish();
    let ldfg = LuauDefinitionFileGenerator::new(definitions);
    let names: Vec<String> = ldfg.iter().map(|(name, _)| name).collect();
    assert_eq!(names, vec!["init.d.luau"]);
}

#[test]
fn test_extension_custom() {
    let definitions = Definitions::start()
        .define("init", Definition::start().register_as("X", Type::string()))
        .finish();
    let ldfg = LuauDefinitionFileGenerator::new(definitions).ext(".luau");
    let names: Vec<String> = ldfg.iter().map(|(name, _)| name).collect();
    assert_eq!(names, vec!["init.luau"]);
}

// ========================
// luau-lsp validation tests
// ========================

#[test]
fn test_luau_lsp_enum_and_value() {
    let out = generate(single(with_value(
        Definition::start().register_as(
            "Direction",
            Type::r#enum([Type::literal("Up"), Type::literal("Down")]),
        ),
        "dir",
        Type::named("Direction"),
        None,
    )));
    validate_with_luau_lsp(
        &out,
        r#"
local _d: Direction = dir
local _u: Direction = "Up"
"#,
    );
}

#[test]
fn test_luau_lsp_alias() {
    let out = generate(single(
        Definition::start().register_as("StringOrNum", Type::string() | Type::number()),
    ));
    validate_with_luau_lsp(
        &out,
        r#"
local _a: StringOrNum = "hello"
local _b: StringOrNum = 42
"#,
    );
}

#[test]
fn test_luau_lsp_function() {
    let out = generate(single(
        Definition::start()
            .param("name", "")
            .function::<String, String>("greet", ()),
    ));
    validate_with_luau_lsp(
        &out,
        r#"
local _result: string = greet("world")
"#,
    );
}

#[test]
fn test_luau_lsp_class_fields_and_methods() {
    let out = generate(single(with_value(
        Definition::start().register_as(
            "Player",
            Type::class(
                TypedClassBuilder::default()
                    .field("name", Type::string(), ())
                    .field("score", Type::integer(), ())
                    .method::<(), String>("getName", ())
                    .build(),
            ),
        ),
        "player",
        Type::named("Player"),
        None,
    )));
    validate_with_luau_lsp(
        &out,
        r#"
local _n: string = player.name
local _s: number = player.score
local _gn: string = player:getName()
"#,
    );
}

#[test]
fn test_luau_lsp_class_with_meta_method() {
    let out = generate(single(with_value(
        Definition::start().register_as(
            "Obj",
            Type::class(
                TypedClassBuilder::default()
                    .field("x", Type::number(), ())
                    .meta_method::<(), String>("__tostring", ())
                    .build(),
            ),
        ),
        "obj",
        Type::named("Obj"),
        None,
    )));
    validate_with_luau_lsp(
        &out,
        r#"
local _s: string = tostring(obj)
local _x: number = obj.x
"#,
    );
}

#[test]
fn test_luau_lsp_optional_type() {
    let out = generate(single(with_value(
        Definition::start().register_as(
            "Container",
            Type::class(
                TypedClassBuilder::default()
                    .field("value", Type::string() | Type::nil(), ())
                    .build(),
            ),
        ),
        "c",
        Type::named("Container"),
        None,
    )));
    assert_eq!(
        out.trim(),
        "declare class Container
\tvalue: string?
end

declare c: Container"
    );
    validate_with_luau_lsp(
        &out,
        r#"
local _v: string? = c.value
"#,
    );
}

#[test]
fn test_luau_lsp_array_type() {
    let out = generate(single(with_value(
        Definition::start().register_as(
            "Holder",
            Type::class(
                TypedClassBuilder::default()
                    .field("items", Type::array(Type::string()), ())
                    .build(),
            ),
        ),
        "h",
        Type::named("Holder"),
        None,
    )));
    validate_with_luau_lsp(
        &out,
        r#"
local _items: {string} = h.items
"#,
    );
}

#[test]
fn test_luau_lsp_map_type() {
    let out = generate(single(with_value(
        Definition::start().register_as(
            "Registry",
            Type::class(
                TypedClassBuilder::default()
                    .field("data", Type::map(Type::string(), Type::number()), ())
                    .build(),
            ),
        ),
        "reg",
        Type::named("Registry"),
        None,
    )));
    validate_with_luau_lsp(
        &out,
        r#"
local _data: {[string]: number} = reg.data
"#,
    );
}

#[test]
fn test_luau_lsp_complex_definition() {
    let out = generate(single(
        with_value(
            Definition::start()
                .register_as(
                    "System",
                    Type::r#enum([Type::literal("Black"), Type::literal("White")]),
                )
                .register_as(
                    "Color",
                    Type::r#enum([Type::named("System"), Type::integer()]),
                )
                .register_as(
                    "Example",
                    Type::class(
                        TypedClassBuilder::default()
                            .field("color", Type::named("Color"), ())
                            .method::<(), String>("describe", ())
                            .meta_method::<(), String>("__tostring", ())
                            .build(),
                    ),
                ),
            "example",
            Type::named("Example"),
            None,
        )
        .param("name", "")
        .function::<String, ()>("greet", ()),
    ));
    validate_with_luau_lsp(
        &out,
        r#"
local _e: Example = example
local _c: Color = _e.color
local _s: string = _e:describe()
local _ts: string = tostring(_e)
greet("world")
"#,
    );
}

// ========================
// Impedance mismatch tests
// ========================
// These tests document known precision losses when mapping the
// mlua-extras Type model to Luau's type system.

/// Variadic<T> erases the inner type to `any` because Luau has no
/// way to express typed variadics in `declare function` signatures
/// (only `...: any` is supported in definition files).
#[test]
fn test_mismatch_variadic_erases_to_any() {
    // Variadic<String> should ideally be `...string` but becomes `any`
    use mlua::Variadic;
    let ty = <Variadic<String> as crate::typed::Typed>::ty();
    assert_eq!(
        ty,
        Type::any(),
        "Variadic<String> should erase to any, losing the String type info"
    );

    // When used as a function parameter, the generated output uses `any`
    let out = generate(single(
        Definition::start().function::<(String, Variadic<i64>), ()>("log", ()),
    ));
    assert_eq!(
        out.trim(),
        "declare function log(param1: string, param2: any): ()"
    );
}

/// Static functions are emitted as a separate global table and can be
/// called without `self` — no impedance mismatch.
#[test]
fn test_luau_lsp_static_functions() {
    let mut builder = TypedClassBuilder::default()
        .field("name", Type::string(), ())
        .method::<(), String>("getName", ());
    builder.typed_class.functions.insert(
        "create".into(),
        Func {
            params: vec![Param {
                name: Some("name".into()),
                ty: Type::string(),
                doc: None,
            }],
            returns: vec![Return {
                ty: Type::named("Player"),
                doc: None,
            }],
            doc: None,
        },
    );
    let out = generate(single(with_value(
        Definition::start().register_as("Player", Type::class(builder.build())),
        "player",
        Type::named("Player"),
        None,
    )));
    validate_with_luau_lsp(
        &out,
        "local p: Player = Player.create(\"alice\")\nlocal _n: string = p:getName()\nlocal _name: string = p.name\n",
    );
}

/// Static (non-self) functions on a class are emitted as a separate
/// `declare ClassName: { ... }` table rather than inside the class body,
/// because `declare class` requires `self` on every function.
#[test]
fn test_static_functions_separate_table() {
    let out = generate(single(
        Definition::start().register_as(
            "Factory",
            Type::class(
                TypedClassBuilder::default()
                    .function::<String, i64>("create", "A static factory method")
                    .build(),
            ),
        ),
    ));
    assert_eq!(
        out.trim(),
        "declare class Factory
end

-- A static factory method
declare function Factory_create(param1: string): number

declare Factory: {
\tcreate: typeof(Factory_create),
}"
    );
}

/// Heterogeneous tuples lose positional type information because Luau
/// has no tuple type syntax. A tuple like (string, integer, boolean)
/// becomes `{ string | integer | boolean }` — an array of the union
/// of all element types, losing the guarantee of which type is at
/// which position.
#[test]
fn test_mismatch_heterogeneous_tuple_loses_position() {
    let out = generate(single(Definition::start().register_as(
        "Record",
        Type::tuple([Type::string(), Type::integer(), Type::boolean()]),
    )));
    // Instead of something like [string, integer, boolean], we get
    // an array whose element type is the union of all tuple types
    assert_eq!(
        out.trim(),
        "export type Record = { string | number | boolean }",
        "Heterogeneous tuple should flatten to union array"
    );
}

/// Luau recognizes `integer` as a type, but numeric literals are inferred
/// as `number` and the two are mutually incompatible — you cannot assign a
/// literal to `integer` or pass `integer` where `number` is expected. The
/// Luau generator maps `Type::integer()` to `number` to avoid this.
#[test]
fn test_integer_maps_to_number() {
    let out = generate(single(with_value(
        Definition::start().register_as(
            "Stats",
            Type::class(
                TypedClassBuilder::default()
                    .field("count", Type::integer(), "An integer field")
                    .field("ratio", Type::number(), "A float field")
                    .build(),
            ),
        ),
        "stats",
        Type::named("Stats"),
        None,
    )));
    assert_eq!(
        out.trim(),
        "declare class Stats
\t-- An integer field
\tcount: number
\t-- A float field
\tratio: number
end

declare stats: Stats"
    );

    // Each field must be consumed with its exact declared type
    validate_with_luau_lsp(
        &out,
        r#"
local _c: number = stats.count
local _r: number = stats.ratio
"#,
    );
}

/// Verifies that Rust integer types (mapped via `Type::integer()`) produce
/// Luau `number` in function signatures and fields, so that passing numeric
/// literals from Luau code does not cause type errors. Without the mapping,
/// `declare function add(a: integer, b: integer): integer` would reject
/// `add(1, 2)` because Luau infers `1` as `number`, and `number` is not
/// assignable to `integer`.
#[test]
fn test_luau_lsp_integer_fields_accept_numeric_literals() {
    let out = generate(single(with_value(
        Definition::start()
            .register_as(
                "Inventory",
                Type::class(
                    TypedClassBuilder::default()
                        .field("count", Type::integer(), ())
                        .field("weight", Type::number(), ())
                        .method::<(i32,), ()>("addItems", ())
                        .method::<(), i64>("total", ())
                        .build(),
                ),
            )
            .param("a", "")
            .param("b", "")
            .function::<(i32, i32), i32>("add", ()),
        "inv",
        Type::named("Inventory"),
        None,
    )));

    // The generated output should use `number` everywhere, not `integer`
    assert!(
        !out.contains("integer"),
        "Luau output should not contain 'integer', got:\n{out}",
    );

    // Numeric literals (which Luau types as `number`) must be accepted
    // without type errors in all positions: function args, method args,
    // return values assigned to variables, and field access.
    validate_with_luau_lsp(
        &out,
        r#"
local _sum: number = add(1, 2)
local _count: number = inv.count
local _weight: number = inv.weight
inv:addItems(5)
local _t: number = inv:total()
"#,
    );
}

/// Enum variants that carry tuple data lose their structure. In the
/// mlua-extras model, an enum variant can be `Type::tuple([Type::integer()])`
/// which flattens to `{ integer }` — an array type. Nested enum
/// variants with different tuple arities all become `{ T }` arrays,
/// losing the distinction between e.g. a 2-element and 3-element variant.
#[test]
fn test_mismatch_enum_tuple_variants_flatten() {
    // An enum where one variant carries a tuple of (integer, string)
    // and another carries just a string
    let out = generate(single(Definition::start().register_as(
        "Payload",
        Type::r#enum([
            Type::literal("None"),
            Type::tuple([Type::integer(), Type::string()]),
            Type::tuple([Type::boolean()]),
        ]),
    )));
    // The tuple variants become union-arrays, losing arity info
    assert_eq!(
        out.trim(),
        "export type Payload = \"None\" | { number | string } | { boolean }"
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
        r#"-- class doc test
declare class TestUserData
	-- attr doc test
	attr: string
	-- method doc test
	function to(self, param1: TestUserData): ()
end

-- function doc test
declare function TestUserData_from(param1: TestUserData): TestUserData

declare TestUserData: {
	from: typeof(TestUserData_from),
}"#
    );
}
