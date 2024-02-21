// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

#[cfg(test)]
mod tests {
    use crate::lru_cache::LruCache;

    #[test]
    fn test_insert_by_get() {
        let capacity = 8usize;
        let mut cache = LruCache::<u64, u64>::new(capacity);

        for i in 0u64..(capacity as u64) {
            assert_eq!(cache.size(), i as usize);
            let j = i;
            let (_, hit) = cache.get(&i, || j + 42);
            assert!(!hit);
        }
        assert_eq!(cache.size(), capacity);

        for i in 0u64..(capacity as u64) {
            let (_, hit) = cache.get(&i, || i + 42);
            assert!(hit, "{}", i);
        }

        for i in (capacity as u64)..((capacity + 5) as u64) {
            let (_, hit) = cache.get(&i, || i + 42);
            assert!(!hit, "{}", i);
        }

        for i in 0u64..5u64 {
            let (_, hit) = cache.get(&i, || i + 42);
            assert!(!hit, "{}", i);
        }

        for i in 5u64..((capacity + 5) as u64) {
            let (_, hit) = cache.get(&i, || i + 42);
            assert!(!hit, "{}", i);
        }

        cache.flush_one(&(capacity as u64));
        assert_eq!(cache.size(), capacity-1);
        assert!(!cache.contains(&(capacity as u64)));
    }
}
