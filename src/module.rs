use crate::store::Store;
use anyhow::{bail, Context, Result};
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

#[derive(Clone, Debug, PartialEq)]
pub enum Instr {
    LocalGet(u32),

    LoadI32(i32),

    I32Add,
    I32Mul,

    Call(u32),
    DivI32U,
    End,
    ConstF64(f64),
}

#[derive(Debug, PartialEq)]
pub struct Func {
    ty: FuncType,
    locals: Vec<Val>,
    pub(crate) body: Vec<Instr>,
}

#[derive(Debug, PartialEq)]
pub struct Export {
    pub(crate) name: String,
    ty: u64,
    pub(crate) idx: u64,
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

        let mut magic = bytes::Buf::take(contents, 4);
        let mut dst = vec![];
        dst.put(&mut magic);
        if dst != MAGIC {
            bail!("wrong magic");
        }

        let contents = magic.into_inner();

        let mut version = bytes::Buf::take(contents, 4);
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

            match section {
                0x01 => {
                    func_types =
                        Self::parse_type_section(&mut contents).context("parse type section")?
                }
                0x03 => {
                    module.funcs = Self::parse_function_section(&mut contents, func_types.clone())
                        .context("parse function section")?
                }
                0x07 => {
                    module.exports =
                        Self::parse_export_section(&mut contents).context("parse export section")?
                }
                0x0A => Self::parse_code_section(&mut contents, &mut module)
                    .context("parse code section")?,
                _ => {
                    let section_len = leb128::read::unsigned(&mut contents)?;
                    let mut t = bytes::Buf::take(contents, section_len as usize);
                    let mut dst = vec![];
                    dst.put(&mut t);
                    contents = t.into_inner();
                    println!("Unknown section id {section}, skipping");
                }
            }
        }

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
                func_type.params.push(Self::parse_val(contents)?);
            }

            let results_len = leb128::read::unsigned(contents)?;
            for _ in 0..results_len {
                func_type.results.push(Self::parse_val(contents)?);
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
            let mut f = module.funcs.get_mut(i as usize).unwrap();

            let func_len = leb128::read::unsigned(&mut contents)?;

            let num_locals = leb128::read::unsigned(&mut contents)?;
            let er = func_len - num_locals - 1;
            let mut locals = Vec::new();
            for _ in 0..num_locals {
                let n = leb128::read::unsigned(&mut contents)?;
                let val = Self::parse_val(contents).context("parse local")?;
                for _ in 0..n {
                    locals.push(val.clone());
                }
            }

            let mut ne = bytes::Buf::take(contents, er as usize);
            let mut b = vec![];
            b.put(&mut ne);
            f.locals = locals;
            f.body = Self::parse_instructions(&mut b.as_ref())?;
            contents = ne.into_inner();
        }

        Ok(())
    }

    fn parse_val(contents: &mut &[u8]) -> Result<Val> {
        let n = contents.get_u8();

        match n {
            0x7F => Ok(Val::I32),
            0x7E => Ok(Val::I64),
            0x7D => Ok(Val::F32),
            0x7C => Ok(Val::F64),
            0x7B => Ok(Val::V128),
            0x70 => Ok(Val::FuncRef),
            0x6F => Ok(Val::ExternRef),
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
                0x00 => continue,
                0x20 => Instr::LocalGet(leb128::read::unsigned(&mut contents)? as u32),
                0x28 => Instr::LoadI32(leb128::read::signed(&mut contents)? as i32),
                0x44 => {
                    let mut name = bytes::Buf::take(contents, 8);
                    let mut n: [u8; 8] = [0; 8];
                    name.copy_to_slice(&mut n);
                    contents = name.into_inner();
                    Instr::ConstF64(f64::from_le_bytes(n))
                }

                0x6A => Instr::I32Add,
                0x6C => Instr::I32Mul,
                0x10 => Instr::Call(leb128::read::unsigned(&mut contents)? as u32),
                0x80 => Instr::DivI32U,
                0x0B => Instr::End,

                _ => {
                    // println!("Unknown opcode {opcode:#x}");
                    continue;
                }
            };

            result.push(instr);
        }

        Ok(result)
    }
}
