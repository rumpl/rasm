use crate::{module::Module, store::Store};
use anyhow::Result;

pub struct Instance;
impl Instance {
    pub(crate) fn new(store: &mut Store, module: &Module) -> Result<Self> {
        Ok(Self {})
    }
}
