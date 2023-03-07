use std::fmt::Display;

use crate::{
    module::{Export, Func, Instr, Module},
    store::Store,
};
use anyhow::{bail, Result};

pub struct Instance {
    pub exports: Exports,
}
impl Instance {
    pub(crate) fn new(_store: &mut Store, module: Module) -> Result<Self> {
        Ok(Self {
            exports: Exports {
                exports: module.exports,
                functions: module.funcs,
            },
        })
    }
}

pub struct Exports {
    exports: Vec<Export>,
    functions: Vec<Func>,
}

impl Exports {
    pub fn get_function(&self, name: &str) -> Result<Function> {
        let mut idx = None;
        for e in &self.exports {
            if e.name == name {
                idx = Some(e.idx);
                break;
            }
        }

        if let Some(idx) = idx {
            let func = self.functions.get(idx as usize).unwrap();
            return Ok(Function {
                body: func.body.clone(),
            });
        }

        bail!("cannot find function {name}");
    }
}

pub struct Function {
    body: Vec<Instr>,
}

impl Function {
    pub fn call(&self, _store: &mut Store, locals: &[Value]) -> Result<Value> {
        let mut stack = vec![];

        for instr in &self.body {
            match instr {
                Instr::LocalGet(n) => stack.push(locals[*n as usize]),
                Instr::I32Add => {
                    let result = self.i32_add(&mut stack)?;
                    stack.push(result);
                }
                Instr::I32Mul => {
                    let result = self.i32_mul(&mut stack)?;
                    stack.push(result);
                }
                Instr::End => break,
            }
        }

        Ok(stack.pop().unwrap())
    }

    fn i32_add(&self, stack: &mut Vec<Value>) -> Result<Value> {
        match (stack.pop(), stack.pop()) {
            (Some(Value::I32(left)), Some(Value::I32(right))) => Ok(Value::I32(left + right)),
            _ => bail!("wrong types for i32_add"),
        }
    }

    fn i32_mul(&self, stack: &mut Vec<Value>) -> Result<Value> {
        match (stack.pop(), stack.pop()) {
            (Some(Value::I32(left)), Some(Value::I32(right))) => {
                Ok(Value::I32(left.saturating_mul(right)))
            }
            _ => bail!("wrong types for i32_add"),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Value {
    I32(i32),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::I32(n) => write!(f, "{n}"),
        }
    }
}
