#[cfg(target_arch = "x86")]
#[macro_use]
pub mod x86_64;

#[cfg(target_arch = "x86")]
pub use x86_64::*;
