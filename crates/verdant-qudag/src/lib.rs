#![no_std]
#![doc = "QuDAG: Quantum-resistant DAG protocol with onion routing for Verdant mesh."]

#[cfg(test)]
extern crate std;

pub mod anonymity;
pub mod bloom;
#[cfg(feature = "std")]
pub mod crypto;
pub mod dag;
pub mod message;
