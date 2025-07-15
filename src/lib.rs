#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

pub mod config;

#[cfg(any(test, feature = "testing"))]
pub mod testing;
