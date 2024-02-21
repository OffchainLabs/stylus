// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use std::hash::Hash;
use fnv::FnvHashMap;

pub struct LruCache<KeyType: Clone, ValueType: Clone> {
    capacity: usize,
    slots: Vec<LruCacheSlot<KeyType, ValueType>>,
    index: FnvHashMap<KeyType, usize>,
    empties: Vec<usize>,
    lru: Option<usize>,
    mru: Option<usize>,
}

#[derive(Clone)]
struct LruCacheSlot<KeyType: Clone, ValueType: Clone> {
    key: KeyType,
    value: ValueType,
    less_ru: Option<usize>,
    more_ru: Option<usize>,
}

impl<KeyType: Clone+PartialEq+Eq+Hash+Default, ValueType: Clone+Default> LruCache<KeyType, ValueType> {
    pub fn new(capacity: usize) -> Self {
        Self{
            capacity,
            slots: vec![LruCacheSlot::default(); capacity],
            index: Default::default(),
            empties: (0..capacity).into_iter().collect(),
            lru: None,
            mru: None,
        }
    }

    pub fn size(&self) -> usize {
        self.capacity - self.empties.len()
    }

    pub fn contains(&self, key: &KeyType) -> bool {
        self.index.contains_key(key)
    }

    pub fn get(&mut self, key: &KeyType, loader: impl Fn() -> ValueType) -> (&ValueType, bool) {
        match self.index.get(key).map(|k| *k) {
            Some(slot_num) => {
                self.promote_slot(slot_num);
                (&self.slots[slot_num].value, true)
            }
            None => {
                let slot_num = self.add_to_cache(key.clone(), loader());
                (&self.slots[slot_num].value, false)
            }
        }
    }

    fn promote_slot(&mut self, slot_num: usize) {
        if let Some(less) = self.slots[slot_num].less_ru {
            self.slots[less].more_ru = self.slots[slot_num].more_ru;
        }
        if let Some(more) = self.slots[slot_num].more_ru {
            self.slots[more].less_ru = self.slots[slot_num].less_ru;
        }
        if self.lru == Some(slot_num) {
            self.lru = self.slots[slot_num].more_ru;
        }
        self.slots[slot_num].more_ru = None;
        self.slots[slot_num].less_ru = self.mru;
        if let Some(less) = self.mru {
            self.slots[less].more_ru = Some(slot_num);
        }
        self.mru = Some(slot_num);
        if self.lru.is_none() {
            self.lru = Some(slot_num);
        }
    }

    fn add_to_cache(&mut self, key: KeyType, value: ValueType) -> usize {
        match self.empties.pop() {
            Some(slot_num) => {
                self.slots[slot_num].key = key.clone();
                self.slots[slot_num].value = value.clone();
                self.slots[slot_num].more_ru = None;
                self.slots[slot_num].less_ru = self.mru;
                self.mru = Some(slot_num);
                if self.lru.is_none() {
                    self.lru = Some(slot_num);
                }
                self.index.insert(key, slot_num);
                slot_num
            }
            None => {
                let slot_num = self.lru.unwrap();
                self.index.remove(&self.slots[slot_num].key);
                self.lru = self.slots[slot_num].more_ru;
                self.slots[slot_num].key = key.clone();
                self.slots[slot_num].value = value.clone();
                self.slots[slot_num].more_ru = None;
                self.slots[self.mru.unwrap()].more_ru = Some(slot_num);
                self.slots[slot_num].less_ru = self.mru;
                self.mru = Some(slot_num);
                self.index.insert(key, slot_num);
                slot_num
            }
        }
    }

    pub fn flush_one(&mut self, key: &KeyType) {
        if let Some(slot_num) = self.index.get(key).map(|k| *k) {
            if self.lru == Some(slot_num) {
                self.lru = self.slots[slot_num].more_ru;
            }
            if self.mru == Some(slot_num) {
                self.mru = self.slots[slot_num].less_ru;
            }
            if let Some(less) = self.slots[slot_num].less_ru {
                self.slots[less].more_ru = self.slots[slot_num].more_ru;
            }
            if let Some(more) = self.slots[slot_num].more_ru {
                self.slots[more].less_ru = self.slots[slot_num].less_ru;
            }
            self.empties.push(slot_num);
            self.index.remove(key);
        }
    }

    pub fn flush_all(&mut self) {
        self.index.clear();
        self.empties = (0..self.capacity).into_iter().collect();
        self.lru = None;
        self.mru = None;
    }
}

impl<KeyType: Clone+Default, ValueType: Clone+Default> Default for LruCacheSlot<KeyType, ValueType> {
    fn default() -> Self {
        Self {
            key: KeyType::default(),
            value: ValueType::default(),
            less_ru: None,
            more_ru: None,
        }
    }
}