#![no_std]
#![doc = "Sensor abstraction layer for Verdant: WiFi CSI, environmental sensors, and fusion."]

#[cfg(test)]
extern crate std;

pub mod csi;
pub mod environmental;
pub mod fusion;
pub mod hal;
