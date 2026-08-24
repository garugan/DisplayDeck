#![deny(unsafe_op_in_unsafe_fn)]

pub mod app_snapshot;

#[cfg(target_os = "windows")]
pub mod candidate;
#[cfg(target_os = "windows")]
pub mod ccd;
#[cfg(target_os = "windows")]
pub mod display;
#[cfg(target_os = "windows")]
pub mod mapping;
#[cfg(target_os = "windows")]
pub mod mutation;
#[cfg(target_os = "windows")]
pub mod observation;
#[cfg(target_os = "windows")]
pub mod qualification;
