#![no_std]
#![doc = "Agentic robotics coordination for Verdant: missions, navigation, safety, swarm."]

#[cfg(test)]
extern crate std;

pub mod mission;
pub mod navigation;
pub mod relay;
pub mod safety;
pub mod swarm;
