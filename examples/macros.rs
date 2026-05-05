use std::path::PathBuf;

use mlua_extras::{
    TypedUserData,
    typed::generator::{
        Definition, DefinitionFileGenerator, Definitions, LuauDefinitionFileGenerator,
    },
    typed_user_data_impl,
};

/// Simple Counter
#[derive(Clone, TypedUserData)]
struct Counter {
    value: i64,
}

#[typed_user_data_impl]
impl Counter {
    /// The default count
    const COUNT: usize = 10;

    /// Max count value
    #[field]
    fn max() -> i64 {
        i64::MAX
    }

    /// Min count value
    #[field(rename = "MIN")]
    fn min() -> i64 {
        0
    }

    /// Direction of the counter
    #[getter("direction")]
    fn get_direction(&self) -> String {
        "up".into()
    }

    #[setter("direction")]
    fn set_direction(&mut self, dir: String) {
        println!("Direction: {dir}");
    }

    /// Get the current counter value
    #[method]
    fn get(&self) -> i64 {
        self.value
    }

    /// Increment the counter
    #[method]
    fn increment(&mut self) {
        self.value += 1
    }

    /// Create a new table
    #[method]
    fn create_table(&self, lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
        lua.create_table()
    }

    /// String representation of the counter
    #[metamethod(ToString)]
    fn to_string(&self) -> String {
        format!("Counter({})", self.value)
    }

    // Requires the `async` feature
    // Must be accessed from lua code with an entry of `mlua::Chunk::eval_async` or `mlua::Chunk::exec_async`

    /// Fetch the global counter online
    #[method]
    async fn fetch(&self, lua: mlua::Lua, url: String) -> mlua::Result<String> {
        _ = lua;
        Ok(format!("fetched: {url}"))
    }
}

fn main() -> mlua::Result<()> {
    let definitions: Definitions = Definitions::start()
        .define("macros", Definition::start().register::<Counter>("Counter"))
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

    Ok(())
}
