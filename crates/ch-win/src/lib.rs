//! Windows implementations (ARCHITECTURE §2): input backends, high-resolution timers, monitors and
//! DPI, global hotkeys, screen capture and window styles.
#![cfg(windows)]

pub mod capture;
pub mod display;
pub mod guard;
pub mod hotkeys;
pub mod input;
pub mod keymap;
pub mod system;
pub mod timing;
