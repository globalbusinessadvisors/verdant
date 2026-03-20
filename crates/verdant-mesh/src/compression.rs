use crate::routing::LinkQuality;

/// Maximum compressed/decompressed buffer size.
const COMP_BUF_SIZE: usize = 8192;

/// Compression level determined by link quality.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompressionLevel {
    /// No compression — good links.
    None,
    /// Light compression — acceptable links.
    Light,
    /// Medium compression — poor links.
    Medium,
    /// Maximum compression — very poor links.
    Maximum,
}

/// Compression errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompressionError {
    InputTooLarge,
    CompressionFailed,
    DecompressionFailed,
}

/// Select compression level based on link quality.
pub fn select_compression(link: &LinkQuality) -> CompressionLevel {
    if link.pdr >= 0.90 {
        CompressionLevel::None
    } else if link.pdr >= 0.70 {
        CompressionLevel::Light
    } else if link.pdr >= 0.50 {
        CompressionLevel::Medium
    } else {
        CompressionLevel::Maximum
    }
}

/// Compress data using LZ4.
///
/// For `None` level, returns the input unchanged. For all other levels,
/// applies LZ4 with a 4-byte uncompressed-length prefix.
pub fn compress(
    data: &[u8],
    level: CompressionLevel,
) -> Result<heapless::Vec<u8, COMP_BUF_SIZE>, CompressionError> {
    if level == CompressionLevel::None {
        return heapless::Vec::from_slice(data).map_err(|_| CompressionError::InputTooLarge);
    }

    let uncomp_len = (data.len() as u32).to_le_bytes();
    let mut comp_buf = [0u8; COMP_BUF_SIZE];
    let comp_len =
        lz4_flex::compress_into(data, &mut comp_buf).map_err(|_| CompressionError::CompressionFailed)?;

    let mut out = heapless::Vec::new();
    // Tag byte: 0x01 = compressed
    out.push(0x01).map_err(|_| CompressionError::InputTooLarge)?;
    out.extend_from_slice(&uncomp_len)
        .map_err(|_| CompressionError::InputTooLarge)?;
    out.extend_from_slice(&comp_buf[..comp_len])
        .map_err(|_| CompressionError::InputTooLarge)?;
    Ok(out)
}

/// Decompress data.
///
/// Detects whether the data is compressed (tag byte prefix) and
/// decompresses accordingly.
pub fn decompress(data: &[u8]) -> Result<heapless::Vec<u8, COMP_BUF_SIZE>, CompressionError> {
    if data.is_empty() {
        return Ok(heapless::Vec::new());
    }

    if data[0] != 0x01 {
        // Not compressed — return as-is
        return heapless::Vec::from_slice(data).map_err(|_| CompressionError::DecompressionFailed);
    }

    if data.len() < 5 {
        return Err(CompressionError::DecompressionFailed);
    }

    let uncomp_len = u32::from_le_bytes([data[1], data[2], data[3], data[4]]) as usize;
    if uncomp_len > COMP_BUF_SIZE {
        return Err(CompressionError::DecompressionFailed);
    }

    let mut decomp_buf = [0u8; COMP_BUF_SIZE];
    let decomp_len = lz4_flex::decompress_into(&data[5..], &mut decomp_buf[..uncomp_len])
        .map_err(|_| CompressionError::DecompressionFailed)?;

    heapless::Vec::from_slice(&decomp_buf[..decomp_len])
        .map_err(|_| CompressionError::DecompressionFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_compression_none_for_good_link() {
        let lq = LinkQuality::from_measurements(0.95, 50);
        assert_eq!(select_compression(&lq), CompressionLevel::None);
    }

    #[test]
    fn select_compression_maximum_for_poor_link() {
        let lq = LinkQuality::from_measurements(0.30, 800);
        assert_eq!(select_compression(&lq), CompressionLevel::Maximum);
    }

    #[test]
    fn compress_none_returns_unchanged() {
        let data = b"hello mesh";
        let result = compress(data, CompressionLevel::None).unwrap();
        assert_eq!(result.as_slice(), data);
    }

    #[test]
    fn compress_decompress_roundtrip() {
        let data = b"The Vermont Living Forest Mesh - ambient intelligence for the land";
        let compressed = compress(data, CompressionLevel::Maximum).unwrap();
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed.as_slice(), data);
    }

    #[test]
    fn decompress_uncompressed_data() {
        let data = b"raw data without compression tag";
        let result = decompress(data).unwrap();
        assert_eq!(result.as_slice(), data);
    }

    #[test]
    fn decompress_empty() {
        let result = decompress(b"").unwrap();
        assert!(result.is_empty());
    }
}
