mod combinator;
mod router;

pub mod prelude {
    pub use crate::combinator::RunVia;
    pub use crate::router::{Router, Via};
}

pub use crate::router::current;
