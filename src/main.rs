use anyhow::Result;
use instance::Instance;
use module::Module;
use store::Store;

mod instance;
mod module;
mod store;

fn main() -> Result<()> {
    let mut store = Store::default();
    let module = Module::from_file(&store, "example.wasm")?;
    let instance = Instance::new(&mut store, &module)?;

    Ok(())
}
