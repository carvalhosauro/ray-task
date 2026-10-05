pub mod clock;
pub mod error;
pub mod model;

pub use clock::{Clock, FixedClock, SystemClock};
pub use error::DomainError;
pub use model::*;
