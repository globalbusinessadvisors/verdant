#![no_std]
#![doc = "SAFLA: Self-Aware Feedback Loop Algorithm for Verdant mesh self-healing."]

#[cfg(test)]
extern crate std;

pub mod consensus;
pub mod events;
pub mod health;
pub mod propagation;
pub mod topology;
