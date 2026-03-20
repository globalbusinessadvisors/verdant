use verdant_core::error::StorageError;
use verdant_core::traits::FlashStorage;
use verdant_core::types::FirmwareHash;

/// Flash partition addresses for dual-bank OTA.
const PARTITION_A_ADDR: u32 = 0x00_0000;
const PARTITION_B_ADDR: u32 = 0x18_0000;
const PARTITION_SIZE: u32 = 0x18_0000; // 1.5 MB

/// OTA update state machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OtaState {
    /// No update in progress.
    Idle,
    /// Receiving firmware chunks.
    Receiving { bytes_written: u32 },
    /// All chunks received, ready to verify.
    ReadyToVerify,
    /// Verified, pending reboot to swap partitions.
    Verified,
    /// Verification failed.
    Failed,
}

/// Manages over-the-air firmware updates using dual-partition flash.
pub struct OtaManager<F: FlashStorage> {
    flash: F,
    active_partition: Partition,
    state: OtaState,
    expected_hash: Option<FirmwareHash>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Partition {
    A,
    B,
}

impl Partition {
    fn inactive(&self) -> Self {
        match self {
            Self::A => Self::B,
            Self::B => Self::A,
        }
    }

    fn base_addr(&self) -> u32 {
        match self {
            Self::A => PARTITION_A_ADDR,
            Self::B => PARTITION_B_ADDR,
        }
    }
}

impl<F: FlashStorage> OtaManager<F> {
    pub fn new(flash: F) -> Self {
        Self {
            flash,
            active_partition: Partition::A,
            state: OtaState::Idle,
            expected_hash: None,
        }
    }

    pub fn state(&self) -> OtaState {
        self.state
    }

    /// Begin an OTA update with the expected firmware hash (from governance vote).
    pub fn begin_update(&mut self, expected_hash: FirmwareHash) {
        self.expected_hash = Some(expected_hash);
        self.state = OtaState::Receiving { bytes_written: 0 };
    }

    /// Write a chunk of firmware to the inactive partition.
    pub fn write_chunk(&mut self, offset: u32, data: &[u8]) -> Result<(), StorageError> {
        match self.state {
            OtaState::Receiving { ref mut bytes_written } => {
                if offset + data.len() as u32 > PARTITION_SIZE {
                    self.state = OtaState::Failed;
                    return Err(StorageError::AddressOutOfRange);
                }
                let target = self.active_partition.inactive();
                self.flash.write_block(target.base_addr() + offset, data)?;
                *bytes_written = offset + data.len() as u32;
                Ok(())
            }
            _ => Err(StorageError::WriteFailed),
        }
    }

    /// Mark reception as complete; transition to verification-ready state.
    pub fn finish_receiving(&mut self) {
        if matches!(self.state, OtaState::Receiving { .. }) {
            self.state = OtaState::ReadyToVerify;
        }
    }

    /// Verify the written firmware against the expected hash.
    ///
    /// In production this reads all blocks and computes SHA-256.
    /// Returns `true` if verification passes.
    pub fn verify(&mut self) -> bool {
        if self.state != OtaState::ReadyToVerify {
            return false;
        }
        // Simplified: in production, read back and hash the inactive partition.
        // For now, assume hash matches if we have an expected hash.
        if self.expected_hash.is_some() {
            self.state = OtaState::Verified;
            true
        } else {
            self.state = OtaState::Failed;
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;

    mock! {
        pub Flash {}
        impl FlashStorage for Flash {
            fn read_block(&self, addr: u32, buf: &mut [u8]) -> Result<(), StorageError>;
            fn write_block(&mut self, addr: u32, data: &[u8]) -> Result<(), StorageError>;
        }
    }

    #[test]
    fn ota_lifecycle() {
        let mut mock = MockFlash::new();
        mock.expect_write_block()
            .times(2) // two chunks
            .returning(|_, _| Ok(()));

        let mut ota = OtaManager::new(mock);
        assert_eq!(ota.state(), OtaState::Idle);

        ota.begin_update(FirmwareHash([0xAB; 32]));
        assert!(matches!(ota.state(), OtaState::Receiving { .. }));

        ota.write_chunk(0, &[1, 2, 3, 4]).unwrap();
        ota.write_chunk(4, &[5, 6, 7, 8]).unwrap();
        ota.finish_receiving();
        assert_eq!(ota.state(), OtaState::ReadyToVerify);

        assert!(ota.verify());
        assert_eq!(ota.state(), OtaState::Verified);
    }

    #[test]
    fn write_chunk_rejects_when_idle() {
        let mock = MockFlash::new();
        let mut ota = OtaManager::new(mock);
        let result = ota.write_chunk(0, &[1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn write_chunk_rejects_out_of_range() {
        let mock = MockFlash::new();
        let mut ota = OtaManager::new(mock);
        ota.begin_update(FirmwareHash([0; 32]));
        let result = ota.write_chunk(PARTITION_SIZE, &[1]);
        assert!(result.is_err());
        assert_eq!(ota.state(), OtaState::Failed);
    }

    #[test]
    fn writes_to_inactive_partition() {
        let mut mock = MockFlash::new();
        // Active is A, so writes should go to B
        mock.expect_write_block()
            .withf(|addr, _| *addr >= PARTITION_B_ADDR)
            .times(1)
            .returning(|_, _| Ok(()));

        let mut ota = OtaManager::new(mock);
        ota.begin_update(FirmwareHash([0; 32]));
        ota.write_chunk(0, &[1, 2, 3, 4]).unwrap();
    }
}
