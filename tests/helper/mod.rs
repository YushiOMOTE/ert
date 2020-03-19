#![allow(unused)]

use ert::current;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Checker {
    map: Arc<Mutex<HashMap<u64, u64>>>,
}

impl Checker {
    pub fn new() -> Self {
        Self {
            map: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add(&self, i: u64) {
        self.map.lock().unwrap().insert(i, current());
    }

    pub fn check(&self, i: u64) {
        assert_eq!(current(), *self.map.lock().unwrap().get(&i).unwrap())
    }
}
