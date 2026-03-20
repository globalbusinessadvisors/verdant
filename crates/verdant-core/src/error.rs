use core::fmt;

/// Errors originating from sensor capture or reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SenseError {
    CsiCaptureTimeout,
    CsiHardwareFault,
    SensorReadFailed,
    CalibrationInvalid,
}

/// Errors originating from mesh transport operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    QueueFull,
    FrameTooLarge,
    EncodingFailed,
    NoRoute,
    Timeout,
}

/// Errors originating from cryptographic operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoError {
    SigningFailed,
    VerificationFailed,
    EncapsulationFailed,
    DecapsulationFailed,
    InvalidKeyLength,
    InvalidSignatureLength,
}

/// Errors originating from flash storage operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    ReadFailed,
    WriteFailed,
    AddressOutOfRange,
    CorruptedData,
    WearLimitExceeded,
}

/// Errors originating from radio hardware.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadioError {
    TransmitFailed,
    ReceiveFailed,
    ScanFailed,
    ChannelBusy,
    HardwareFault,
}

/// Errors originating from governance operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GovernanceError {
    UnknownZone,
    DuplicateVote,
    ProposalNotFound,
    DeadlinePassed,
    InvalidQuorum,
    InsufficientCredits,
}

/// Errors originating from confirmed event emission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmitError {
    BroadcastFailed,
    SerializationFailed,
}

/// Errors originating from network healing operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealError {
    RerouteFailed,
    NoAlternativePath,
    TopologyChangeRejected,
}

/// Unified error type aggregating all domain errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerdantError {
    Sense(SenseError),
    Transport(TransportError),
    Crypto(CryptoError),
    Storage(StorageError),
    Radio(RadioError),
    Governance(GovernanceError),
    Emit(EmitError),
    Heal(HealError),
}

impl fmt::Display for VerdantError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sense(e) => write!(f, "Sense: {e:?}"),
            Self::Transport(e) => write!(f, "Transport: {e:?}"),
            Self::Crypto(e) => write!(f, "Crypto: {e:?}"),
            Self::Storage(e) => write!(f, "Storage: {e:?}"),
            Self::Radio(e) => write!(f, "Radio: {e:?}"),
            Self::Governance(e) => write!(f, "Governance: {e:?}"),
            Self::Emit(e) => write!(f, "Emit: {e:?}"),
            Self::Heal(e) => write!(f, "Heal: {e:?}"),
        }
    }
}

macro_rules! impl_from {
    ($variant:ident, $inner:ty) => {
        impl From<$inner> for VerdantError {
            fn from(e: $inner) -> Self {
                Self::$variant(e)
            }
        }
    };
}

impl_from!(Sense, SenseError);
impl_from!(Transport, TransportError);
impl_from!(Crypto, CryptoError);
impl_from!(Storage, StorageError);
impl_from!(Radio, RadioError);
impl_from!(Governance, GovernanceError);
impl_from!(Emit, EmitError);
impl_from!(Heal, HealError);
