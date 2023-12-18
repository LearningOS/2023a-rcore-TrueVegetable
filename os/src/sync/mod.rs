//! Synchronization and interior mutability primitives

mod up;

pub use up::UPSafeCell;
pub use up::UUPSafeCell;