//! Optional in-RAM LRU cache. Default OFF (no `cache_size` in searxng.conf
//! means disabled). Fixed slot count, LRU evict — never grows unbounded.

use std::collections::{HashMap, VecDeque};

pub struct Cache {
    cap: usize,
    map: HashMap<String, String>,
    order: VecDeque<String>,
}

#[allow(dead_code)]
impl Cache {
    pub fn new(cap_entries: usize) -> Self {
        Self {
            cap: cap_entries,
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    pub fn disabled() -> Self {
        Self::new(0)
    }

    pub fn get(&mut self, key: &str) -> Option<String> {
        if self.cap == 0 {
            return None;
        }
        self.map.get(key).cloned()
    }

    pub fn put(&mut self, key: String, value: String) {
        if self.cap == 0 {
            return;
        }
        if self.map.contains_key(&key) {
            return;
        }
        if self.map.len() >= self.cap {
            if let Some(old) = self.order.pop_front() {
                self.map.remove(&old);
            }
        }
        self.order.push_back(key.clone());
        self.map.insert(key, value);
    }
}
