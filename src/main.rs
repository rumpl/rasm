use anyhow::Result;
use instance::{Instance, Value};
use module::Module;
use store::Store;

mod instance;
mod module;
mod store;

fn main() -> Result<()> {
    let mut store = Store::default();
    let module = Module::from_file(&store, "example2.wasm")?;
    let instance = Instance::new(&mut store, module)?;
    let add = instance.exports.get_function("add")?;
    let result = add.call(&mut store, &[Value::I32(12), Value::I32(42), Value::I32(2)])?;

    println!("{result}");

    Ok(())
}
