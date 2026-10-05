//! CH. Clicker domain (ARCHITECTURE §2): models, schema, movement patterns, state machine and a
//! scheduler over an abstract clock. Pure Rust with no Windows or Tauri dependency, so everything
//! here is testable on any OS.

pub mod geometry;
pub mod input;

pub const PRODUCT_NAME: &str = "CH. Clicker";
