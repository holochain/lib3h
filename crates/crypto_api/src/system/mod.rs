use crate::{CryptoRandom, CryptoSignature};

pub trait CryptoSystem: CryptoRandom + CryptoSignature {
    fn get() -> &'static Self;
}
