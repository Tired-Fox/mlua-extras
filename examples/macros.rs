use std::path::PathBuf;

use mlua_extras::{
    TypedUserData,
    typed::generator::{
        Definition, DefinitionFileGenerator, Definitions, LuauDefinitionFileGenerator,
    },
    typeduserdata_impl,
};

/// Simple Counter
#[derive(Clone, TypedUserData)]
struct Counter {
    value: i64,
}

#[typeduserdata_impl]
impl Counter {
    /// The default count
    #[lua(field)]
    const COUNT: usize = 10;

    /// Max count value
    #[lua(field, name = "MAX", infallible)]
    fn max() -> i64 {
        i64::MAX
    }

    /// Min count value
    #[lua(field, name = "MIN", infallible)]
    fn min() -> i64 {
        0
    }

    /// Direction of the counter
    #[lua(getter, name = "direction", infallible)]
    fn get_direction(&self) -> String {
        "up".into()
    }

    #[lua(setter, name = "direction", infallible)]
    fn set_direction(&mut self, dir: String) {
        println!("Direction: {dir}");
    }

    /// Get the current counter value
    #[lua(infallible)]
    fn get(&self) -> i64 {
        self.value
    }

    /// Increment the counter
    #[lua(infallible)]
    fn increment(&mut self) {
        self.value += 1
    }

    /// Create a new table
    fn create_table(&self, lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
        lua.create_table()
    }

    /// String representation of the counter
    #[lua(meta, infallible)]
    fn __tostring(&self) -> String {
        format!("Counter({})", self.value)
    }

    // Requires the `async` feature
    // Must be accessed from lua code with an entry of `mlua::Chunk::eval_async` or `mlua::Chunk::exec_async`

    /// Fetch the global counter online
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
