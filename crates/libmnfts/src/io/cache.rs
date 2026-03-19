use std::collections::{HashMap, VecDeque};

/// Simple LRU cache for block data.
pub struct BlockCache {
    capacity: usize,
    map: HashMap<u64, Vec<u8>>,
    order: VecDeque<u64>,
}

impl BlockCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            map: HashMap::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
        }
    }

    pub fn get(&mut self, block_num: u64) -> Option<&Vec<u8>> {
        if self.map.contains_key(&block_num) {
            self.order.retain(|&b| b != block_num);
            self.order.push_back(block_num);
            self.map.get(&block_num)
        } else {
            None
        }
    }

    pub fn put(&mut self, block_num: u64, data: Vec<u8>) {
        if self.map.contains_key(&block_num) {
            self.order.retain(|&b| b != block_num);
        } else if self.map.len() >= self.capacity
            && let Some(evicted) = self.order.pop_front()
        {
            self.map.remove(&evicted);
        }
        self.map.insert(block_num, data);
        self.order.push_back(block_num);
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }
}
