//! CH. Clicker domain (ARCHITECTURE §2): models, schema, movement patterns, state machine and a
//! scheduler over an abstract clock. Pure Rust with no Windows or Tauri dependency, so everything
//! here is testable on any OS.

pub mod easing;
pub mod engine;
pub mod geometry;
pub mod input;
pub mod model;
pub mod schema;
pub mod stats;
pub mod time;
pub mod validate;

pub const PRODUCT_NAME: &str = "CH. Clicker";
