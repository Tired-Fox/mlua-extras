# CHANGELOG

##### 0.0.6

**Features**

- Add the `ExtendModule` trait to allow tables to be extended by the contents created from the `Module` trait
- Add `Index` enum to handle to handle indexing types
- Add `lua` lifetime to `add_*_function` and `add_*_method` methods to allow for lua type construction and return. i.e. tables, strings, etc.
- Definition file generation handles table key escaping. i.e. `[0]` and `["key.name"]`
- Rename `Type::Struct` to `Type::Table`
- Add better `BitOr` support for anything that can be converted to a `Type`
    - This allows for cool type building ex: `Type::nil() | "literal" | 3 | true | [Type::string(), Type::integer()]`
- Simplify registering types with the definiton builder

**Fixes**

- Typed derive macro using wrong traits
- Export missing traits and objects

##### 0.0.5

**Features**

- Define typed lua modules
- Change each item to only have a single doc comment
- Restucture definition builder
- Add `*_with` method syntax to allow for additional documentation
- Update readme to better reflect the project
- Add mlua features and conditionally expose api with those features
- Restructure crate around exposing mlua becuase of its limitations
- Add gitignore for derive create
- Update naming and doc comments

**Fixes**

- Set lua to be vendored for when docs.rs is generated
- Docs.rs generation
- Fix readme typos

##### 0.0.2

- Restructure library around exporting `mlua`
- Update `README` to include and example of using the `typed` module

##### 0.0.1

- This version is not meant to be used in production, but is release to start being used to figure out what should be changed, removed, and added.
- Initial beta version with helper macros, helper traits, and lua types.
