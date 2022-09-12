// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use wasmer::{ExportIndex, Function, GlobalInit, GlobalType, Instance, Mutability, Type};
use wasmer_types::{GlobalIndex, ModuleInfo, Value};

use std::{convert::TryInto, fmt::Debug};

pub fn add_global(module: &mut ModuleInfo, name: &str, ty: Type, init: GlobalInit) -> GlobalIndex {
    let name = format!("polyglot_{name}");
    let global_type = GlobalType::new(ty, Mutability::Var);
    let index = module.globals.push(global_type);
    module.exports.insert(name, ExportIndex::Global(index));
    module.global_initializers.push(init);
    index
}

pub fn get_global<T>(instance: &Instance, name: &str) -> T
where
    T: TryFrom<Value<Function>>,
    T::Error: Debug,
{
    let error = format!("global {name} does not exist");
    let global = instance.exports.get_global(name).expect(&error);
    global.get().try_into().expect("wrong type")
}

pub fn set_global<T>(instance: &Instance, name: &str, value: T)
where
    T: Into<Value<Function>>,
{
    let error = format!("global {name} does not exist");
    let global = instance.exports.get_global(name).expect(&error);
    global.set(value.into()).expect("failed to write global");
}
