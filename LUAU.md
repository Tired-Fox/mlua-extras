# Luau Definition File Generator

mlua-extras can generate `.d.luau` definition files that provide type
information to the Luau type checker and [luau-lsp](https://github.com/JohnnyMorganz/luau-lsp).
These files use native Luau type syntax (`declare class`, `declare function`,
`export type`, etc.) rather than the LuaLS `--- @` annotation style used by
the existing `DefinitionFileGenerator`.

## Quick start

```rust
use mlua_extras::typed::generator::{
    Definition, Definitions, LuauDefinitionFileGenerator,
};

let definitions = Definitions::start()
    .define("init", Definition::start()
        .register::<MyType>("MyType")
        .value::<MyType>("instance")
        .param("name", "Person to greet")
        .function::<String, ()>("greet", ())
    )
    .finish();

let gen = LuauDefinitionFileGenerator::new(definitions);
for (name, writer) in gen.iter() {
    writer.write_file(format!("types/{name}")).unwrap();
}
```

This produces a file like:

```luau
declare class MyType
    name: string
    function getName(self): string
end

declare instance: MyType

declare function greet(name: string): ()
```

## Validating generated definitions

The standalone `luau-analyze` CLI from the [Luau repo](https://github.com/luau-lang/luau)
does **not** support loading custom definition files. Use
[luau-lsp](https://github.com/JohnnyMorganz/luau-lsp) instead, which has a
one-shot `analyze` subcommand:

```bash
# Install luau-lsp (Linux x86_64 example)
curl -L -o luau-lsp.zip \
  https://github.com/JohnnyMorganz/luau-lsp/releases/latest/download/luau-lsp-linux-x86_64.zip
unzip luau-lsp.zip && chmod +x luau-lsp

# Type-check a script against your generated definitions
luau-lsp analyze --defs=@myapp=types/init.d.luau my_script.luau
```

The `@myapp=` prefix is an arbitrary namespace label. Multiple `--defs` flags
can be passed to load several definition files.

## Luau definition file syntax

The Luau [grammar page](https://luau.org/grammar) documents the language
syntax but omits the `declare` keyword family used in definition files. This
syntax is parsed by the Luau frontend but is only meaningful when loaded
through the definition file API (as luau-lsp does with `--defs`). The
standalone `luau-analyze` CLI treats `declare` statements as syntax errors
when encountered in regular source files.

### `declare class`

Declares a named class type with fields and methods:

```luau
declare class Player
    name: string
    score: integer
    function getName(self): string
    function addScore(self, amount: integer): ()
end
```

Every `function` inside a `declare class` block **must** list `self` as its
first parameter (unannotated — no `: Type` after it). There is no way to
declare a static or class-level function inside the class body. See
[Impedance mismatches](#impedance-mismatches) below for how this affects the
generator.

Inheritance is supported:

```luau
declare class Animal
    name: string
end

declare class Dog extends Animal
    breed: string
end
```

(The mlua-extras `Type` model does not currently support `extends`.)

### `export type`

Declares a named type alias visible to consumers of the definition file.
Without `export`, type aliases are file-local and not usable by scripts that
load the definitions.

```luau
export type Color = "Red" | "Green" | "Blue"
export type StringOrNum = string | number
export type Callback = (name: string) -> boolean
```

### `declare function`

Declares a global function:

```luau
declare function greet(name: string): ()
declare function parse(input: string): (boolean, string)
```

### `declare` (global variable)

Declares a global variable with a type:

```luau
declare myGlobal: string
declare config: { host: string, port: integer }
```

### Type syntax differences from LuaLS

| Concept | LuaLS | Luau |
|---|---|---|
| Function type | `fun(a: T): R` | `(a: T) -> R` |
| Array type | `T[]` | `{T}` |
| Map type | `{ [K]: V }` | `{[K]: V}` |
| Optional | `T \| nil` | `T?` (sugar) |
| Intersection | not supported | `A & B` |

## Impedance mismatches

The following are known precision losses when mapping the mlua-extras `Type`
model to Luau's type system. Each is covered by a test in
`src/typed/generator/luau_type_file_tests.rs`.

### `Variadic<T>` erases to `any`

Rust's `Variadic<String>` maps to `Type::any()`, producing `...: any` in the
definition file. The inner type is lost. A function declared as
`fn(String, Variadic<i64>)` generates:

```luau
declare function log(param0: string, param1: any): ()
```

Ideally this would be `...: string` but the `Type` model does not yet support
typed variadics or type packs.

### Static functions use a separate global table

Luau's `declare class` requires every `function` to have `self` as its first
parameter, so there is no way to express a true static function inside the
class body. The generator emits static functions (registered via
`add_function`) as a separate `declare ClassName: { ... }` global table:

```luau
declare class Factory
end

declare Factory: {
    create: (name: string) -> integer,
}
```

This allows consumers to call `Factory.create("x")` without a spurious
`self` parameter. The class declaration provides the type, and the global
table declaration provides the static function bindings. Both coexist on the
same name.

### Heterogeneous tuples lose positional typing

Luau has no tuple type syntax. A Rust tuple like `(String, i64, bool)` becomes
an array whose element type is the union of all members:

```luau
export type Record = { string | integer | boolean }
```

This allows any element type at any position. Homogeneous tuples like
`(i64, i64, i64)` correctly collapse to `{ integer }`.

### `integer` and `number` are distinct

Luau treats `integer` and `number` as incompatible types in definition files.
Assigning an `integer`-typed value to a `number`-annotated variable (or vice
versa) is a type error:

```luau
declare class Stats
    count: integer
    ratio: number
end
```

```luau
-- This is a type error in luau-lsp:
local n: number = stats.count
```

The mlua-extras model maps Rust integer types (`i32`, `u64`, etc.) to
`integer` and float types (`f32`, `f64`) to `number`, which is faithful to
Luau's type system but may surprise users who expect them to be
interchangeable.

### Enum tuple variants flatten

Rust enum variants that carry tuple data lose their structure. A variant like
`Color::Rgb(u8, u8, u8)` becomes `{ integer }` — an array type with no arity
constraint. An enum with mixed variant shapes:

```rust
Type::r#enum([
    Type::literal("None"),
    Type::tuple([Type::integer(), Type::string()]),
    Type::tuple([Type::boolean()]),
])
```

generates:

```luau
export type Payload = "None" | { integer | string } | { boolean }
```

The distinction between a 2-element `(integer, string)` variant and a
3-element variant of the same types is lost.

### Features not yet in the `Type` model

The following Luau type system features cannot be expressed with the current
mlua-extras `Type` enum and would require model changes to support:

- **Generics** — `declare function clone<T>(value: T): T`
- **Type packs** — `declare function pcall<A..., R...>(f: (A...) -> R..., ...: A...): (boolean, R...)`
- **Class inheritance** — `declare class Dog extends Animal`
- **Intersection types** — `((string) -> number) & ((number) -> string)`
