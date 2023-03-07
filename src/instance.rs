use crate::{module::Module, store::Store};
use anyhow::Result;

pub struct Instance;
impl Instance {
    pub(crate) fn new(_store: &mut Store, _module: &Module) -> Result<Self> {
        Ok(Self {})
    }
}
