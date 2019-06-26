use crate::{CryptoRandom, CryptoSignature};

/// CryptoSystem pulls our crypto sub-system traits together
pub trait CryptoSystem: CryptoRandom + CryptoSignature {}
