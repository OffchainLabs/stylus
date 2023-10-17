// Copyright 2021-2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

use crate::{
    programs::{
        config::CompileConfig, counter::Counter, depth::DepthChecker, dynamic::DynamicMeter,
        heap::HeapBound, meter::Meter, start::StartMover, FuncMiddleware, Middleware, ModuleMod,
        StylusData, STYLUS_ENTRY_POINT,
    },
    value::{ArbValueType, FunctionType, IntegerValType, Value},
};
use arbutil::{math::SaturatingSum, Color, DebugColor};
use eyre::{bail, ensure, eyre, Result, WrapErr};
use fnv::{FnvHashMap as HashMap, FnvHashSet as HashSet};
use nom::{
    branch::alt,
    bytes::complete::tag,
    combinator::{all_consuming, map, value},
    sequence::{preceded, tuple},
};
use serde::{Deserialize, Serialize};
use std::{convert::TryInto, fmt::Debug, hash::Hash, mem, path::Path, str::FromStr};
use wasmer_types::{entity::EntityRef, FunctionIndex, LocalFunctionIndex};
use wasmparser::{
    Data, Element, Export, ExternalKind, Global, Import, ImportSectionEntryType, MemoryType, Name,
    NameSectionReader, Naming, Operator, Parser, Payload, TableType, Type, TypeDef, Validator,
    WasmFeatures,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FloatType {
    F32,
    F64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FloatUnOp {
    Abs,
    Neg,
    Ceil,
    Floor,
    Trunc,
    Nearest,
    Sqrt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FloatBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Min,
    Max,
    CopySign,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FloatRelOp {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FloatInstruction {
    UnOp(FloatType, FloatUnOp),
    BinOp(FloatType, FloatBinOp),
    RelOp(FloatType, FloatRelOp),
    /// The bools represent (saturating, signed)
    TruncIntOp(IntegerValType, FloatType, bool, bool),
    ConvertIntOp(FloatType, IntegerValType, bool),
    F32DemoteF64,
    F64PromoteF32,
}

impl FloatInstruction {
    pub fn signature(&self) -> FunctionType {
        match *self {
            FloatInstruction::UnOp(t, _) => FunctionType::new([t.into()], [t.into()]),
            FloatInstruction::BinOp(t, _) => FunctionType::new([t.into(); 2], [t.into()]),
            FloatInstruction::RelOp(t, _) => FunctionType::new([t.into(); 2], [ArbValueType::I32]),
            FloatInstruction::TruncIntOp(i, f, ..) => FunctionType::new([f.into()], [i.into()]),
            FloatInstruction::ConvertIntOp(f, i, _) => FunctionType::new([i.into()], [f.into()]),
            FloatInstruction::F32DemoteF64 => {
                FunctionType::new([ArbValueType::F64], [ArbValueType::F32])
            }
            FloatInstruction::F64PromoteF32 => {
                FunctionType::new([ArbValueType::F32], [ArbValueType::F64])
            }
        }
    }
}

impl FromStr for FloatInstruction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        type IResult<'a, T> = nom::IResult<&'a str, T, nom::error::Error<&'a str>>;

        fn parse_fp_type(s: &str) -> IResult<FloatType> {
            alt((
                value(FloatType::F32, tag("f32")),
                value(FloatType::F64, tag("f64")),
            ))(s)
        }

        fn parse_signedness(s: &str) -> IResult<bool> {
            alt((value(true, tag("s")), value(false, tag("u"))))(s)
        }

        fn parse_int_type(s: &str) -> IResult<IntegerValType> {
            alt((
                value(IntegerValType::I32, tag("i32")),
                value(IntegerValType::I64, tag("i64")),
            ))(s)
        }

        fn parse_un_op(s: &str) -> IResult<FloatUnOp> {
            alt((
                value(FloatUnOp::Abs, tag("abs")),
                value(FloatUnOp::Neg, tag("neg")),
                value(FloatUnOp::Ceil, tag("ceil")),
                value(FloatUnOp::Floor, tag("floor")),
                value(FloatUnOp::Trunc, tag("trunc")),
                value(FloatUnOp::Nearest, tag("nearest")),
                value(FloatUnOp::Sqrt, tag("sqrt")),
            ))(s)
        }

        fn parse_bin_op(s: &str) -> IResult<FloatBinOp> {
            alt((
                value(FloatBinOp::Add, tag("add")),
                value(FloatBinOp::Sub, tag("sub")),
                value(FloatBinOp::Mul, tag("mul")),
                value(FloatBinOp::Div, tag("div")),
                value(FloatBinOp::Min, tag("min")),
                value(FloatBinOp::Max, tag("max")),
                value(FloatBinOp::CopySign, tag("copysign")),
            ))(s)
        }

        fn parse_rel_op(s: &str) -> IResult<FloatRelOp> {
            alt((
                value(FloatRelOp::Eq, tag("eq")),
                value(FloatRelOp::Ne, tag("ne")),
                value(FloatRelOp::Lt, tag("lt")),
                value(FloatRelOp::Gt, tag("gt")),
                value(FloatRelOp::Le, tag("le")),
                value(FloatRelOp::Ge, tag("ge")),
            ))(s)
        }

        let inst = alt((
            map(
                all_consuming(tuple((parse_fp_type, tag("_"), parse_un_op))),
                |(t, _, o)| FloatInstruction::UnOp(t, o),
            ),
            map(
                all_consuming(tuple((parse_fp_type, tag("_"), parse_bin_op))),
                |(t, _, o)| FloatInstruction::BinOp(t, o),
            ),
            map(
                all_consuming(tuple((parse_fp_type, tag("_"), parse_rel_op))),
                |(t, _, o)| FloatInstruction::RelOp(t, o),
            ),
            map(
                all_consuming(tuple((
                    parse_int_type,
                    alt((
                        value(true, tag("_trunc_sat_")),
                        value(false, tag("_trunc_")),
                    )),
                    parse_fp_type,
                    tag("_"),
                    parse_signedness,
                ))),
                |(i, sat, f, _, s)| FloatInstruction::TruncIntOp(i, f, sat, s),
            ),
            map(
                all_consuming(tuple((
                    parse_fp_type,
                    tag("_convert_"),
                    parse_int_type,
                    tag("_"),
                    parse_signedness,
                ))),
                |(f, _, i, _, s)| FloatInstruction::ConvertIntOp(f, i, s),
            ),
            value(
                FloatInstruction::F32DemoteF64,
                all_consuming(tag("f32_demote_f64")),
            ),
            value(
                FloatInstruction::F64PromoteF32,
                all_consuming(tag("f64_promote_f32")),
            ),
        ));

        let res = preceded(tag("wavm__"), inst)(s);

        res.map(|(_, i)| i).map_err(|e| e.to_string())
    }
}

pub fn op_as_const(op: Operator) -> Result<Value> {
    match op {
        Operator::I32Const { value } => Ok(Value::I32(value as u32)),
        Operator::I64Const { value } => Ok(Value::I64(value as u64)),
        Operator::F32Const { value } => Ok(Value::F32(f32::from_bits(value.bits()))),
        Operator::F64Const { value } => Ok(Value::F64(f64::from_bits(value.bits()))),
        _ => bail!("Opcode is not a constant"),
    }
}

#[derive(Clone, Debug, Default)]
pub struct FuncImport<'a> {
    pub offset: u32,
    pub module: &'a str,
    pub name: Option<&'a str>, // in wasmparser 0.95+ this won't be optional
}

/// This enum primarily exists because wasmer's ExternalKind doesn't impl these derived functions
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportKind {
    Func,
    Table,
    Memory,
    Global,
    Tag,
}

impl TryFrom<ExternalKind> for ExportKind {
    type Error = eyre::Error;

    fn try_from(kind: ExternalKind) -> Result<Self> {
        use ExternalKind::*;
        match kind {
            Function => Ok(Self::Func),
            Table => Ok(Self::Table),
            Memory => Ok(Self::Memory),
            Global => Ok(Self::Global),
            Tag => Ok(Self::Tag),
            kind => bail!("unsupported kind {:?}", kind),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Code<'a> {
    pub locals: Vec<Local>,
    pub expr: Vec<Operator<'a>>,
}

#[derive(Clone, Debug)]
pub struct Local {
    pub index: u32,
    pub value: ArbValueType,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameCustomSection {
    pub module: String,
    pub functions: HashMap<u32, String>,
}

pub type ExportMap = HashMap<String, (u32, ExportKind)>;

#[derive(Clone, Default)]
pub struct WasmBinary<'a> {
    pub types: Vec<FunctionType>,
    pub imports: Vec<FuncImport<'a>>,
    /// Maps *local* function indices to global type signatures
    pub functions: Vec<u32>,
    pub tables: Vec<TableType>,
    pub memories: Vec<MemoryType>,
    pub globals: Vec<Value>,
    pub exports: ExportMap,
    pub start: Option<u32>,
    pub elements: Vec<Element<'a>>,
    pub codes: Vec<Code<'a>>,
    pub datas: Vec<Data<'a>>,
    pub names: NameCustomSection,
}

pub fn parse<'a>(input: &'a [u8], path: &'_ Path) -> Result<WasmBinary<'a>> {
    let features = WasmFeatures {
        mutable_global: true,
        saturating_float_to_int: true,
        sign_extension: true,
        reference_types: false,
        multi_value: true,
        bulk_memory: true, // not all ops supported yet
        module_linking: false,
        simd: false,
        relaxed_simd: false,
        threads: false,
        tail_call: false,
        deterministic_only: false,
        multi_memory: false,
        exceptions: false,
        memory64: false,
        extended_const: false,
    };
    let mut validator = Validator::new();
    validator.wasm_features(features);
    validator
        .validate_all(input)
        .wrap_err_with(|| eyre!("failed to validate {}", path.to_string_lossy().red()))?;

    let mut binary = WasmBinary::default();
    let sections: Vec<_> = Parser::new(0).parse_all(input).collect::<Result<_, _>>()?;

    for mut section in sections {
        use Payload::*;

        macro_rules! process {
            ($dest:expr, $source:expr) => {{
                for _ in 0..$source.get_count() {
                    let item = $source.read()?;
                    $dest.push(item.into())
                }
            }};
        }
        macro_rules! flatten {
            ($ty:tt, $source:expr) => {{
                let mut values: Vec<$ty> = Vec::new();
                for _ in 0..$source.get_count() {
                    let item = $source.read()?;
                    values.push(item.into())
                }
                values
            }};
        }

        match &mut section {
            TypeSection(type_section) => {
                for _ in 0..type_section.get_count() {
                    let ty = match type_section.read()? {
                        TypeDef::Func(ty) => ty,
                        x => bail!("Unsupported type section {:?}", x),
                    };
                    binary.types.push(ty.try_into()?);
                }
            }
            CodeSectionEntry(codes) => {
                let mut code = Code::default();
                let mut locals = codes.get_locals_reader()?;
                let mut ops = codes.get_operators_reader()?;
                let mut index = 0;

                for _ in 0..locals.get_count() {
                    let (count, value) = locals.read()?;
                    for _ in 0..count {
                        code.locals.push(Local {
                            index,
                            value: value.try_into()?,
                        });
                        index += 1;
                    }
                }
                while !ops.eof() {
                    code.expr.push(ops.read()?);
                }

                binary.codes.push(code);
            }
            GlobalSection(globals) => {
                for global in flatten!(Global, globals) {
                    let mut init = global.init_expr.get_operators_reader();

                    let value = match (init.read()?, init.read()?, init.eof()) {
                        (op, Operator::End, true) => op_as_const(op)?,
                        _ => bail!("Non-constant global initializer"),
                    };
                    binary.globals.push(value);
                }
            }
            ImportSection(imports) => {
                for import in flatten!(Import, imports) {
                    let ImportSectionEntryType::Function(offset) = import.ty else {
                        bail!("unsupported import kind {:?}", import)
                    };
                    let import = FuncImport {
                        offset,
                        module: import.module,
                        name: import.field,
                    };
                    binary.imports.push(import);
                }
            }
            ExportSection(exports) => {
                use ExternalKind::*;
                for export in flatten!(Export, exports) {
                    let name = export.field.to_owned();
                    if let Function = export.kind {
                        let index = export.index;
                        let name = || name.clone();
                        binary.names.functions.entry(index).or_insert_with(name);
                    }

                    // TODO: we'll only support the types also in wasmparser 0.95+
                    if matches!(export.kind, Function | Table | Memory | Global | Tag) {
                        let kind = export.kind.try_into()?;
                        binary.exports.insert(name, (export.index, kind));
                    } else {
                        bail!("unsupported export kind {:?}", export)
                    }
                }
            }
            FunctionSection(functions) => process!(binary.functions, functions),
            TableSection(tables) => process!(binary.tables, tables),
            MemorySection(memories) => process!(binary.memories, memories),
            StartSection { func, .. } => binary.start = Some(*func),
            ElementSection(elements) => process!(binary.elements, elements),
            DataSection(datas) => process!(binary.datas, datas),
            CodeSectionStart { .. } => {}
            CustomSection {
                name,
                data_offset,
                data,
                ..
            } => {
                if *name != "name" {
                    continue;
                }

                let mut name_reader = NameSectionReader::new(data, *data_offset)?;

                while !name_reader.eof() {
                    match name_reader.read()? {
                        Name::Module(name) => binary.names.module = name.get_name()?.to_owned(),
                        Name::Function(namemap) => {
                            let mut map_reader = namemap.get_map()?;
                            for _ in 0..map_reader.get_count() {
                                let Naming { index, name } = map_reader.read()?;
                                binary.names.functions.insert(index, name.to_owned());
                            }
                        }
                        _ => {}
                    }
                }
            }
            Version { num, .. } => ensure!(*num == 1, "wasm format version not supported {}", num),
            UnknownSection { id, .. } => bail!("unsupported unknown section type {}", id),
            End => {}
            x => bail!("unsupported section type {:?}", x),
        }
    }

    // reject the module if it imports the same func with inconsistent signatures
    let mut imports = HashMap::default();
    for import in &binary.imports {
        let offset = import.offset;
        let module = import.module;
        let name = import.name;

        let key = (module, name);
        if let Some(prior) = imports.insert(key, offset) {
            if prior != offset {
                let name = name.debug_red();
                bail!("inconsistent imports for {} {name}", module.red());
            }
        }
    }

    // reject the module if it re-exports an import with the same name
    let mut exports = HashSet::default();
    for export in binary.exports.keys() {
        let export = export.rsplit("__").take(1);
        exports.extend(export);
    }
    for import in &binary.imports {
        if let Some(name) = import.name {
            if exports.contains(name) {
                bail!("binary exports an import with the same name {}", name.red());
            }
        }
    }

    // reject the module if it imports or exports reserved symbols
    let reserved = |x: &&str| x.starts_with("stylus");
    if let Some(name) = exports.into_iter().find(reserved) {
        bail!("binary exports reserved symbol {}", name.red())
    }
    if let Some(name) = binary.imports.iter().filter_map(|x| x.name).find(reserved) {
        bail!("binary imports reserved symbol {}", name.red())
    }

    // if no module name was given, make a best-effort guess with the file path
    if binary.names.module.is_empty() {
        binary.names.module = match path.file_name() {
            Some(os_str) => os_str.to_string_lossy().into(),
            None => path.to_string_lossy().into(),
        };
    }
    Ok(binary)
}

impl<'a> Debug for WasmBinary<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmBinary")
            .field("types", &self.types)
            .field("imports", &self.imports)
            .field("functions", &self.functions)
            .field("tables", &self.tables)
            .field("memories", &self.memories)
            .field("globals", &self.globals)
            .field("exports", &self.exports)
            .field("start", &self.start)
            .field("elements", &format!("<{} elements>", self.elements.len()))
            .field("codes", &self.codes)
            .field("datas", &self.datas)
            .field("names", &self.names)
            .finish()
    }
}

impl<'a> WasmBinary<'a> {
    /// Instruments a user wasm, producing a version bounded via configurable instrumentation.
    pub fn instrument(&mut self, compile: &CompileConfig) -> Result<StylusData> {
        let meter = Meter::new(compile.pricing.costs);
        let dygas = DynamicMeter::new(&compile.pricing);
        let depth = DepthChecker::new(compile.bounds);
        let bound = HeapBound::new(compile.bounds);
        let start = StartMover::default();

        meter.update_module(self)?;
        dygas.update_module(self)?;
        depth.update_module(self)?;
        bound.update_module(self)?;
        start.update_module(self)?;

        let count = compile.debug.count_ops.then(Counter::new);
        if let Some(count) = &count {
            count.update_module(self)?;
        }

        for (index, code) in self.codes.iter_mut().enumerate() {
            let index = LocalFunctionIndex::from_u32(index as u32);
            let locals: Vec<Type> = code.locals.iter().map(|x| x.value.into()).collect();

            let mut build = mem::take(&mut code.expr);
            let mut input = Vec::with_capacity(build.len());

            /// this macro exists since middlewares aren't sized (can't use a vec without boxes)
            macro_rules! apply {
                ($middleware:expr) => {
                    let mut mid = Middleware::<WasmBinary>::instrument(&$middleware, index)?;
                    mid.locals_info(&locals);

                    mem::swap(&mut build, &mut input);

                    for op in input.drain(..) {
                        mid.feed(op, &mut build)
                            .wrap_err_with(|| format!("{} failure", mid.name()))?
                    }
                };
            }

            // add the instrumentation in the order of application
            // note: this must be consistent with native execution
            apply!(meter);
            apply!(dygas);
            apply!(depth);
            apply!(bound);
            apply!(start);

            if let Some(count) = &count {
                apply!(*count);
            }

            code.expr = build;
        }

        // 4GB maximum implies `footprint` fits in a u16
        let footprint = self.memory_info()?.min.0 as u16;

        // check the entrypoint
        let ty = FunctionType::new([ArbValueType::I32], [ArbValueType::I32]);
        let user_main = self.check_func(STYLUS_ENTRY_POINT, ty)?;

        let [ink_left, ink_status] = meter.globals();
        let depth_left = depth.globals();
        Ok(StylusData {
            ink_left,
            ink_status,
            depth_left,
            footprint,
            user_main,
        })
    }

    /// Parses and instruments a user wasm
    pub fn parse_user(
        wasm: &'a [u8],
        page_limit: u16,
        compile: &CompileConfig,
    ) -> Result<(WasmBinary<'a>, StylusData, u16)> {
        let mut bin = parse(wasm, Path::new("user"))?;
        let stylus_data = bin.instrument(compile)?;

        let Some(memory) = bin.memories.first() else {
            bail!("missing memory with export name \"memory\"")
        };
        let pages = memory.initial;

        // ensure the wasm fits within the remaining amount of memory
        if pages > page_limit.into() {
            let limit = page_limit.red();
            bail!("memory exceeds limit: {} > {limit}", pages.red());
        }

        // not strictly necessary, but anti-DoS limits and extra checks in case of bugs
        macro_rules! limit {
            ($limit:expr, $count:expr, $name:expr) => {
                if $count > $limit {
                    bail!("too many wasm {}: {} > {}", $name, $count, $limit);
                }
            };
        }
        limit!(1, bin.memories.len(), "memories");
        limit!(100, bin.datas.len(), "datas");
        limit!(100, bin.elements.len(), "elements");
        limit!(1_000, bin.exports.len(), "exports");
        limit!(1_000, bin.tables.len(), "tables");
        limit!(10_000, bin.codes.len(), "functions");
        limit!(50_000, bin.globals.len(), "globals");
        for function in &bin.codes {
            limit!(4096, function.locals.len(), "locals")
        }

        let table_entries = bin.tables.iter().map(|x| x.initial).saturating_sum();
        limit!(10_000, table_entries, "table entries");

        let max_len = 500;
        macro_rules! too_long {
            ($name:expr, $len:expr) => {
                bail!(
                    "wasm {} too long: {} > {}",
                    $name.red(),
                    $len.red(),
                    max_len.red()
                )
            };
        }
        if let Some((name, _)) = bin.exports.iter().find(|(name, _)| name.len() > max_len) {
            too_long!("name", name.len())
        }
        if bin.names.module.len() > max_len {
            too_long!("module name", bin.names.module.len())
        }
        if bin.start.is_some() {
            bail!("wasm start functions not allowed");
        }
        Ok((bin, stylus_data, pages as u16))
    }

    /// Ensures a func exists and has the right type.
    fn check_func(&self, name: &str, ty: FunctionType) -> Result<u32> {
        let Some(&(func, kind)) = self.exports.get(name) else {
            bail!("missing export with name {}", name.red());
        };
        if kind != ExportKind::Func {
            let kind = kind.debug_red();
            bail!("export {} must be a function but is a {kind}", name.red());
        }
        let func_ty = self.get_function(FunctionIndex::new(func.try_into()?))?;
        if func_ty != ty {
            bail!("wrong type for {}: {}", name.red(), func_ty.red());
        }
        Ok(func)
    }
}
