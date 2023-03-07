use crate::store::Store;
use anyhow::{bail, Result};
use bytes::{Buf, BufMut};
use std::path::Path;

static MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6D];
static VERSION: [u8; 4] = [0x01, 0x00, 0x00, 0x00];

#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    // Num types
    I32,
    I64,
    F32,
    F64,

    // Vec type
    V128,

    // Ref type
    FuncRef,
    ExternRef,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct FuncType {
    pub params: Vec<Val>,
    pub results: Vec<Val>,
}

#[derive(Debug, PartialEq)]
pub enum Instr {
    LocalGet(u32),

    I32Add,
    End,
}

#[derive(Debug, PartialEq)]
pub struct Func {
    ty: FuncType,
    locals: Vec<Val>,
    body: Vec<Instr>,
}

#[derive(Debug, PartialEq)]
pub struct Export {
    name: String,
    ty: u64,
    idx: u64,
}

#[derive(Debug, PartialEq, Default)]
pub struct Module {
    pub funcs: Vec<Func>,
    pub exports: Vec<Export>,
}

impl Module {
    pub fn from_file<T>(_store: &Store, file: T) -> Result<Self>
    where
        T: AsRef<Path>,
    {
        let mut module = Self::default();
        let contents = std::fs::read(file)?;

        let contents: &[u8] = contents.as_ref();

        let mut magic = contents.take(4);
        let mut dst = vec![];
        dst.put(&mut magic);
        if dst != MAGIC {
            bail!("wrong magic");
        }

        let contents = magic.into_inner();

        let mut version = contents.take(4);
        let mut dst = vec![];
        dst.put(&mut version);

        if dst != VERSION {
            bail!("wrong version");
        }

        let mut contents = version.into_inner();

        let mut func_types = Vec::new();
        loop {
            if contents.remaining() == 0 {
                break;
            }
            let section = contents.get_u8();

            println!("section {section}");
            match section {
                0x0 => bail!("meh"),
                0x01 => func_types = Self::parse_type_section(&mut contents)?,
                0x03 => {
                    module.funcs = Self::parse_function_section(&mut contents, func_types.clone())?
                }
                0x07 => module.exports = Self::parse_export_section(&mut contents)?,
                0x0A => Self::parse_code_section(&mut contents, &mut module)?,
                _ => break,
            }
        }

        println!("{module:?}");

        Ok(module)
    }

    fn parse_type_section(mut contents: &mut &[u8]) -> Result<Vec<FuncType>> {
        let _section_len = leb128::read::unsigned(&mut contents)?;
        let types_len = leb128::read::unsigned(&mut contents)?;

        let mut result = Vec::new();

        for _ in 0..types_len {
            let mut func_type = FuncType::default();

            // 0x60, start of functype
            let start = contents.get_u8();
            if start != 0x60 {
                bail!("malformed module, expected start of functype (0x60), got {start}");
            }

            let params_len = leb128::read::unsigned(&mut contents)?;
            for _ in 0..params_len {
                func_type.params.push(Self::parse_val(&mut contents)?);
            }

            let results_len = leb128::read::unsigned(&mut contents)?;
            for _ in 0..results_len {
                func_type.results.push(Self::parse_val(&mut contents)?);
            }

            result.push(func_type);
        }

        Ok(result)
    }

    fn parse_function_section(
        mut contents: &mut &[u8],
        func_types: Vec<FuncType>,
    ) -> Result<Vec<Func>> {
        let _section_len = leb128::read::unsigned(&mut contents)?;

        let function_len = leb128::read::unsigned(&mut contents)?;
        let mut result = Vec::new();
        for _ in 0..function_len {
            let idx = leb128::read::unsigned(&mut contents)?;
            if let Some(ty) = func_types.get(idx as usize) {
                result.push(Func {
                    ty: ty.clone(),
                    locals: Vec::new(),
                    body: Vec::new(),
                });
            } else {
                bail!("Unable to find function type {}", idx);
            }
        }

        Ok(result)
    }

    fn parse_export_section(mut contents: &mut &[u8]) -> Result<Vec<Export>> {
        let mut result = Vec::new();

        let _section_len = leb128::read::unsigned(&mut contents)?;
        let num_exports = leb128::read::unsigned(&mut contents)?;

        for _ in 0..num_exports {
            let n = leb128::read::unsigned(&mut contents)?;

            let mut name = bytes::Buf::take(contents, n as usize);
            let mut n = vec![];
            n.put(&mut name);

            contents = name.into_inner();

            let name = String::from_utf8(n)?;
            let ty = leb128::read::unsigned(&mut contents)?;
            let idx = leb128::read::unsigned(&mut contents)?;

            result.push(Export { name, ty, idx })
        }

        Ok(result)
    }

    fn parse_code_section(mut contents: &mut &[u8], module: &mut Module) -> Result<()> {
        let _section_len = leb128::read::unsigned(&mut contents)?;

        let n = leb128::read::unsigned(&mut contents)?;
        for i in 0..n {
            let _func_len = leb128::read::unsigned(&mut contents)?;

            let num_locals = leb128::read::unsigned(&mut contents)?;
            let mut locals = Vec::new();
            for _ in 0..num_locals {
                locals.push(Self::parse_val(contents)?);
            }

            let mut f = module.funcs.get_mut(i as usize).unwrap();
            f.locals = locals;
            f.body = Self::parse_instructions(contents)?;
        }

        Ok(())
    }

    fn parse_val(contents: &mut &[u8]) -> Result<Val> {
        let n = contents.get_u8();

        match n {
            127 => Ok(Val::I32),
            _ => bail!("unknown type {n}"),
        }
    }

    fn parse_instructions(mut contents: &mut &[u8]) -> Result<Vec<Instr>> {
        let mut result = Vec::new();

        loop {
            if contents.remaining() == 0 {
                break;
            }
            let opcode = contents.get_u8();

            let instr = match opcode {
                0x20 => Instr::LocalGet(leb128::read::unsigned(&mut contents)? as u32),
                0x6A => Instr::I32Add,
                0x0B => Instr::End,
                _ => bail!("Unknown opcode {opcode:#x}"),
            };

            result.push(instr);
        }

        Ok(result)
    }
}
