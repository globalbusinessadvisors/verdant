#![no_std]
#![doc = "ESP32-S3 node firmware for the Verdant mesh."]
#![doc = ""]
#![doc = "Tests run on x86 with mocked hardware traits. Production builds"]
#![doc = "target `xtensa-esp32s3-none-elf` with concrete ESP32 implementations."]

#[cfg(test)]
extern crate std;

pub mod cycle;
pub mod ota;
pub mod power;
pub mod storage;
