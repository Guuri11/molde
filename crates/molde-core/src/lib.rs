//! Dominio y casos de uso, sin IO: no conoce LSP, Treesitter ni SQLite.

pub mod domain;
pub mod ports;
pub mod use_cases;

pub use domain::*;
pub use ports::*;
pub use use_cases::*;
