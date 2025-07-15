#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

pub mod config;
pub mod logging;

#[cfg(any(test, feature = "testing"))]
pub mod testing;
