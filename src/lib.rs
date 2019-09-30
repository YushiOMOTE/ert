//! ert
//!
//! A combinator to control future execution order.
//!

mod combinator;
mod router;
mod utils;

pub use crate::combinator::RunVia;
pub use crate::router::Router;
pub use crate::utils::worker_id;

pub mod prelude {
    pub use super::{worker_id, Router, RunVia};
}
