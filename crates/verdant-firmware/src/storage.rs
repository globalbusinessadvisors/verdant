use verdant_core::error::StorageError;
use verdant_core::traits::FlashStorage;

/// Flash partition addresses.
pub const VECTOR_GRAPH_ADDR: u32 = 0x10_0000;
pub const CONFIG_ADDR: u32 = 0x1C_0000;
pub const EVENT_LOG_ADDR: u32 = 0x1D_0000;

/// Block size for flash write operations.
pub const BLOCK_SIZE: usize = 4096;

/// Manages persisting vector graph and configuration to flash.
pub struct StorageManager<F: FlashStorage> {
    flash: F,
}

impl<F: FlashStorage> StorageManager<F> {
    pub fn new(flash: F) -> Self {
        Self { flash }
    }

    /// Write a block of data to the vector graph partition.
    pub fn write_graph_block(&mut self, offset: u32, data: &[u8]) -> Result<(), StorageError> {
        self.flash.write_block(VECTOR_GRAPH_ADDR + offset, data)
    }

    /// Read a block from the vector graph partition.
    pub fn read_graph_block(&self, offset: u32, buf: &mut [u8]) -> Result<(), StorageError> {
        self.flash.read_block(VECTOR_GRAPH_ADDR + offset, buf)
    }

    /// Write configuration data.
    pub fn write_config(&mut self, data: &[u8]) -> Result<(), StorageError> {
        self.flash.write_block(CONFIG_ADDR, data)
    }

    /// Read configuration data.
    pub fn read_config(&self, buf: &mut [u8]) -> Result<(), StorageError> {
        self.flash.read_block(CONFIG_ADDR, buf)
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
    fn write_graph_block_uses_correct_address() {
        let mut mock = MockFlash::new();
        mock.expect_write_block()
            .withf(|addr, _| *addr == VECTOR_GRAPH_ADDR + 0x100)
            .times(1)
            .returning(|_, _| Ok(()));

        let mut mgr = StorageManager::new(mock);
        mgr.write_graph_block(0x100, &[1, 2, 3]).unwrap();
    }

    #[test]
    fn read_graph_block_uses_correct_address() {
        let mut mock = MockFlash::new();
        mock.expect_read_block()
            .withf(|addr, _| *addr == VECTOR_GRAPH_ADDR)
            .times(1)
            .returning(|_, _| Ok(()));

        let mgr = StorageManager::new(mock);
        let mut buf = [0u8; 64];
        mgr.read_graph_block(0, &mut buf).unwrap();
    }

    #[test]
    fn write_config_uses_config_address() {
        let mut mock = MockFlash::new();
        mock.expect_write_block()
            .withf(|addr, _| *addr == CONFIG_ADDR)
            .times(1)
            .returning(|_, _| Ok(()));

        let mut mgr = StorageManager::new(mock);
        mgr.write_config(&[0xDE, 0xAD]).unwrap();
    }
}
