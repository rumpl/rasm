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
pub struct Instr {}

#[derive(Debug, PartialEq)]
pub struct Func {
    ty: FuncType,
    locals: Vec<Val>,
    body: Vec<Instr>,
}

#[derive(Debug, PartialEq)]
pub struct Export {}

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
            let section = contents.get_u8();

            println!("section {section}");
            match section {
                0x0 => bail!("meh"),
                0x01 => func_types = Self::parse_type_section(&mut contents)?,
                0x03 => {
                    module.funcs = Self::parse_function_section(&mut contents, func_types.clone())?
                }
                0x07 => module.exports = Self::parse_export_section(&mut contents)?,
                _ => break,
            }

            println!("{module:?}");
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
            result.push(Func {
                ty: func_types.get(idx as usize).unwrap().clone(), // TODO: remove the unwrap here
                locals: Vec::new(),
                body: Vec::new(),
            })
        }

        Ok(result)
    }

    fn parse_export_section(mut contents: &mut &[u8]) -> Result<Vec<Export>> {
        let a = contents.get_u8();
        println!("section {a}");
        let n = leb128::read::unsigned(&mut contents)?;
        println!("section length: {n}");

        let n = leb128::read::unsigned(&mut contents)?;
        println!("vec length: {n}");
        let n = leb128::read::unsigned(&mut contents)?;
        println!("func length: {n}");

        let n = leb128::read::unsigned(&mut contents)?;
        println!("num locals: {n}");

        // contents is the body
        println!("{contents:#x?}");
        Ok(Vec::new())
    }

    fn parse_val(contents: &mut &mut &[u8]) -> Result<Val> {
        let n = contents.get_u8();

        match n {
            127 => Ok(Val::I32),
            _ => bail!("unknown type"),
        }
    }
}
