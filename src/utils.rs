use futures::task_local;
use std::cell::Cell;

task_local! {
    static WORKER_ID: Cell<usize> = Cell::new(0)
}

pub fn set_worker_id(id: usize) {
    WORKER_ID.with(|wid| wid.set(id))
}

pub fn worker_id() -> usize {
    WORKER_ID.with(|wid| wid.get())
}
