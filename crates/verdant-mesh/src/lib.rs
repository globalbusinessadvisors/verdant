#![no_std]
#![doc = "Mesh networking layer for Verdant: discovery, routing, transport, partition recovery."]

#[cfg(test)]
extern crate std;

pub mod compression;
pub mod discovery;
pub mod partition;
pub mod routing;
pub mod transport;
