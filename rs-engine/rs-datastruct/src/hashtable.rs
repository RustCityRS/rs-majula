use std::ops::{Index, IndexMut};

struct HashEntry<T> {
    value: Option<T>,
    key: i64,
    prev: usize,
    next: usize,
}

#[allow(clippy::len_without_is_empty)]
pub struct HashTable<T> {
    revision: u64,
    bucket_count: usize,
    entries: Vec<HashEntry<T>>,
    free: Vec<usize>,
    len: usize,
}

impl<T> HashTable<T> {
    pub fn new(bucket_count: usize) -> Self {
        let mut entries = Vec::with_capacity(bucket_count);
        for i in 0..bucket_count {
            entries.push(HashEntry {
                value: None,
                key: 0,
                prev: i,
                next: i,
            });
        }
        HashTable {
            revision: 0,
            bucket_count,
            entries,
            free: Vec::new(),
            len: 0,
        }
    }

    fn alloc(&mut self, key: i64, value: T) -> usize {
        if let Some(idx) = self.free.pop() {
            self.entries[idx] = HashEntry {
                value: Some(value),
                key,
                prev: 0,
                next: 0,
            };
            idx
        } else {
            let idx = self.entries.len();
            self.entries.push(HashEntry {
                value: Some(value),
                key,
                prev: 0,
                next: 0,
            });
            idx
        }
    }

    pub fn get(&self, key: i64) -> Option<usize> {
        let sentinel = (key as usize) & (self.bucket_count - 1);
        let mut cur = self.entries[sentinel].next;
        while cur != sentinel {
            if self.entries[cur].key == key {
                return Some(cur);
            }
            cur = self.entries[cur].next;
        }
        None
    }

    pub fn put(&mut self, key: i64, value: T) -> usize {
        self.revision = self.revision.wrapping_add(1);
        let idx = self.alloc(key, value);
        let sentinel = (key as usize) & (self.bucket_count - 1);
        let prev = self.entries[sentinel].prev;
        self.entries[idx].prev = prev;
        self.entries[idx].next = sentinel;
        self.entries[prev].next = idx;
        self.entries[sentinel].prev = idx;
        self.len += 1;
        idx
    }

    pub fn key(&self, handle: usize) -> i64 {
        self.entries[handle].key
    }

    pub fn value(&self, handle: usize) -> &T {
        self.entries[handle].value.as_ref().expect("invalid handle")
    }

    pub fn value_mut(&mut self, handle: usize) -> &mut T {
        self.revision = self.revision.wrapping_add(1);
        self.entries[handle].value.as_mut().expect("invalid handle")
    }

    pub fn unlink(&mut self, handle: usize) -> T {
        self.revision = self.revision.wrapping_add(1);
        let prev = self.entries[handle].prev;
        let next = self.entries[handle].next;
        self.entries[prev].next = next;
        self.entries[next].prev = prev;
        let value = self.entries[handle].value.take().expect("double unlink");
        self.free.push(handle);
        self.len -= 1;
        value
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn iter(&self) -> Iter<'_, T> {
        let current = if self.bucket_count > 0 {
            self.entries[0].next
        } else {
            0
        };
        Iter {
            table: self,
            bucket: 0,
            current,
        }
    }
}

pub struct Iter<'a, T> {
    table: &'a HashTable<T>,
    bucket: usize,
    current: usize,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.bucket >= self.table.bucket_count {
                return None;
            }
            if self.current == self.bucket {
                self.bucket += 1;
                if self.bucket < self.table.bucket_count {
                    self.current = self.table.entries[self.bucket].next;
                }
                continue;
            }
            let entry = &self.table.entries[self.current];
            self.current = entry.next;
            return entry.value.as_ref();
        }
    }
}

impl<T> Index<usize> for HashTable<T> {
    type Output = T;

    fn index(&self, handle: usize) -> &T {
        self.value(handle)
    }
}

impl<T> IndexMut<usize> for HashTable<T> {
    fn index_mut(&mut self, handle: usize) -> &mut T {
        self.value_mut(handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_and_get() {
        let mut table = HashTable::new(8);
        table.put(42, "hello");
        let handle = table.get(42).unwrap();
        assert_eq!(table[handle], "hello");
        assert_eq!(table.key(handle), 42);
    }

    #[test]
    fn get_missing_key() {
        let table: HashTable<i32> = HashTable::new(8);
        assert_eq!(table.get(99), None);
    }

    #[test]
    fn multiple_keys_same_bucket() {
        let mut table = HashTable::new(8);
        table.put(1, "a");
        table.put(9, "b"); // same bucket as 1 (9 & 7 == 1)
        table.put(17, "c"); // same bucket (17 & 7 == 1)

        assert_eq!(table[table.get(1).unwrap()], "a");
        assert_eq!(table[table.get(9).unwrap()], "b");
        assert_eq!(table[table.get(17).unwrap()], "c");
    }

    #[test]
    fn unlink_and_get() {
        let mut table = HashTable::new(8);
        let h = table.put(10, "x");
        assert_eq!(table.unlink(h), "x");
        assert_eq!(table.get(10), None);
    }

    #[test]
    fn unlink_middle_of_chain() {
        let mut table = HashTable::new(8);
        table.put(1, "a");
        let h2 = table.put(9, "b");
        table.put(17, "c");

        table.unlink(h2);
        assert_eq!(table.get(9), None);
        assert_eq!(table[table.get(1).unwrap()], "a");
        assert_eq!(table[table.get(17).unwrap()], "c");
    }

    #[test]
    fn reuse_freed_slots() {
        let mut table = HashTable::new(4);
        let h = table.put(1, 100);
        table.unlink(h);
        table.put(2, 200);
        assert_eq!(table[table.get(2).unwrap()], 200);
    }

    #[test]
    fn index_mut() {
        let mut table = HashTable::new(8);
        let h = table.put(5, 10);
        table[h] = 20;
        assert_eq!(table[h], 20);
    }

    #[test]
    fn negative_key() {
        let mut table = HashTable::new(8);
        table.put(-5, "neg");
        let h = table.get(-5).unwrap();
        assert_eq!(table[h], "neg");
    }

    #[test]
    fn iter_bucket_order() {
        let mut table = HashTable::new(4);
        table.put(0, "a"); // bucket 0
        table.put(1, "b"); // bucket 1
        table.put(4, "c"); // bucket 0 (4 & 3 == 0)
        table.put(2, "d"); // bucket 2

        let values: Vec<&str> = table.iter().copied().collect();
        assert_eq!(values, vec!["a", "c", "b", "d"]);
        assert_eq!(table.len(), 4);
    }

    #[test]
    fn iter_after_unlink() {
        let mut table = HashTable::new(4);
        table.put(0, 10);
        let h = table.put(1, 20);
        table.put(2, 30);

        table.unlink(h);
        let values: Vec<i32> = table.iter().copied().collect();
        assert_eq!(values, vec![10, 30]);
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn iter_empty() {
        let table: HashTable<i32> = HashTable::new(8);
        assert_eq!(table.iter().count(), 0);
        assert_eq!(table.len(), 0);
    }

    #[test]
    fn many_entries() {
        let mut table = HashTable::new(16);
        for i in 0..100i64 {
            table.put(i, i * 10);
        }
        for i in 0..100i64 {
            let h = table.get(i).unwrap();
            assert_eq!(table[h], i * 10);
        }
    }
}
