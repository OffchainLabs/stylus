// Copyright 2021-2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use crate::memory::{Memory, EMPTY_MEM_HASH};
use arbutil::Bytes32;
use digest::Digest;
use lazy_static::lazy_static;
use sha3::Keccak256;
use std::collections::HashMap;
use std::sync::RwLock;

#[cfg(feature = "native")]
use rayon::prelude::*;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum MerkleType {
    Empty,
    Value,
    Function,
    FunctionType,
    Opcode,
    ArgumentData,
    Memory,
    Table,
    TableElement,
    Module,
}

impl Default for MerkleType {
    fn default() -> Self {
        Self::Empty
    }
}

impl MerkleType {
    pub fn get_prefix(self) -> &'static str {
        match self {
            MerkleType::Empty => panic!("Attempted to get prefix of empty merkle type"),
            MerkleType::Value => "Value merkle tree:",
            MerkleType::Function => "Function merkle tree:",
            MerkleType::FunctionType => "Function type merkle tree:",
            MerkleType::Opcode => "Opcode merkle tree:",
            MerkleType::ArgumentData => "Argument data merkle tree:",
            MerkleType::Memory => "Memory merkle tree:",
            MerkleType::Table => "Table merkle tree:",
            MerkleType::TableElement => "Table element merkle tree:",
            MerkleType::Module => "Module merkle tree:",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Merkle {
    ty: MerkleType,
    layers: Vec<Vec<Bytes32>>,
    min_depth: usize,
}

fn hash_node(ty: MerkleType, a: Bytes32, b: Bytes32) -> Bytes32 {
    let mut h = Keccak256::new();
    h.update(ty.get_prefix());
    h.update(a);
    h.update(b);
    h.finalize().into()
}

lazy_static! {
    static ref EMPTY_LAYERS: RwLock<HashMap<MerkleType, RwLock<Vec<Bytes32>>>> = Default::default();
}

impl Merkle {
    fn get_empty_readonly(ty: MerkleType, layer: usize) -> Option<Bytes32> {
        match EMPTY_LAYERS.read().unwrap().get(&ty) {
            None => None,
            Some(rwvec) => rwvec.read().unwrap().get(layer).copied(),
        }
    }

    pub fn get_empty(ty: MerkleType, layer: usize) -> Bytes32 {
        let exists = Self::get_empty_readonly(ty, layer);
        if let Some(val_exists) = exists {
            return val_exists;
        }
        let new_val: Bytes32;
        if layer == 0 {
            new_val = match ty {
                MerkleType::Empty => {
                    panic!("attempted to fetch empty-layer value from empty merkle")
                }
                MerkleType::Memory => *EMPTY_MEM_HASH,
                _ => Bytes32::default(),
            }
        } else {
            let prev_val = Self::get_empty(ty, layer - 1);
            new_val = hash_node(ty, prev_val, prev_val);
        }
        let mut layers = EMPTY_LAYERS.write().unwrap();
        let mut typed = layers.entry(ty).or_default().write().unwrap();
        if typed.len() > layer {
            assert_eq!(typed[layer], new_val);
        } else if typed.len() == layer {
            typed.push(new_val);
        } else {
            panic!("trying to compute empty merkle entries out of order")
        }
        return typed[layer];
    }

    pub fn new(ty: MerkleType, hashes: Vec<Bytes32>) -> Merkle {
        if hashes.is_empty() {
            return Merkle::default();
        }
        let min_depth = match ty {
            MerkleType::Empty => panic!("attempted to fetch empty-layer value from empty merkle"),
            MerkleType::Memory => Memory::MEMORY_LAYERS,
            MerkleType::Opcode => 2,
            MerkleType::ArgumentData => 2,
            _ => 0,
        };
        let mut layers = vec![hashes];
        while layers.last().unwrap().len() > 1 || layers.len() < min_depth {
            let empty_layer = Self::get_empty(ty, layers.len() - 1);
            let next_empty_layer = Self::get_empty(ty, layers.len());

            #[cfg(feature = "native")]
            let new_layer = layers.last().unwrap().par_chunks(2);

            #[cfg(not(feature = "native"))]
            let new_layer = layers.last().unwrap().chunks(2);

            let new_layer = new_layer
                .map(|chunk| {
                    let left = chunk[0];
                    let right = chunk.get(1).cloned().unwrap_or(empty_layer);
                    if left == empty_layer && right == empty_layer {
                        next_empty_layer
                    } else {
                        hash_node(ty, left, right)
                    }
                })
                .collect();
            layers.push(new_layer);
        }
        Merkle {
            ty,
            layers,
            min_depth,
        }
    }

    pub fn root(&self) -> Bytes32 {
        if let Some(layer) = self.layers.last() {
            assert_eq!(layer.len(), 1);
            layer[0]
        } else {
            Bytes32::default()
        }
    }

    pub fn leaves(&self) -> &[Bytes32] {
        if self.layers.is_empty() {
            &[]
        } else {
            &self.layers[0]
        }
    }

    #[must_use]
    pub fn prove(&self, idx: usize) -> Option<Vec<u8>> {
        if idx >= self.leaves().len() {
            return None;
        }
        Some(self.prove_any(idx))
    }

    /// creates a merkle proof regardless of if the leaf has content
    #[must_use]
    pub fn prove_any(&self, mut idx: usize) -> Vec<u8> {
        let mut proof = vec![u8::try_from(self.layers.len() - 1).unwrap()];
        for (layer_i, layer) in self.layers.iter().enumerate() {
            if layer_i == self.layers.len() - 1 {
                break;
            }
            let counterpart = idx ^ 1;
            proof.extend(
                layer
                    .get(counterpart)
                    .cloned()
                    .unwrap_or_else(|| Self::get_empty(self.ty, layer_i)),
            );
            idx >>= 1;
        }
        proof
    }

    /// Adds a new leaf to the merkle
    /// Currently O(n) in the number of leaves (could be log(n))
    pub fn push_leaf(&mut self, leaf: Bytes32) {
        let mut leaves = self.layers.swap_remove(0);
        leaves.push(leaf);
        *self = Self::new(self.ty, leaves);
    }

    /// Removes the rightmost leaf from the merkle
    /// Currently O(n) in the number of leaves (could be log(n))
    pub fn pop_leaf(&mut self) {
        let mut leaves = self.layers.swap_remove(0);
        leaves.pop();
        *self = Self::new(self.ty, leaves);
    }

    pub fn set(&mut self, mut idx: usize, hash: Bytes32) {
        if self.layers[0][idx] == hash {
            return;
        }
        let mut next_hash = hash;
        let layers_len = self.layers.len();
        for (layer_i, layer) in self.layers.iter_mut().enumerate() {
            layer[idx] = next_hash;
            if layer_i == layers_len - 1 {
                // next_hash isn't needed
                break;
            }
            let counterpart = layer
                .get(idx ^ 1)
                .cloned()
                .unwrap_or_else(|| Self::get_empty(self.ty, layer_i));
            if idx % 2 == 0 {
                next_hash = hash_node(self.ty, next_hash, counterpart);
            } else {
                next_hash = hash_node(self.ty, counterpart, next_hash);
            }
            idx >>= 1;
        }
    }
}
