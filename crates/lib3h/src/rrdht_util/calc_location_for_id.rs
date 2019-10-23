use crate::{error::Lib3hResult, rrdht_util::Location};
use lib3h_crypto_api::{Buffer, CryptoSystem};

#[allow(clippy::borrowed_box)]
/// Given a space-layer agent_id, or a transport/node-layer node_id
/// the ID must be an HCID string form. E.g. "HcSyada..." or "HcMyada..."
/// calculate the circular rrdht "location" u32 value.
pub fn calc_location_for_id(crypto: &Box<dyn CryptoSystem>, id: &str) -> Lib3hResult<Location> {
    // get an hcid encoder for the appropriate input
    let enc = match id.chars().nth(2) {
        Some('S') | Some('s') => hcid::HcidEncoding::with_kind("hcs0")?,
        Some('M') | Some('m') => hcid::HcidEncoding::with_kind("hcm0")?,
        _ => return Err(format!("invalid hcid: {}", id).into()),
    };

    // first, get the raw bytes out of the hcid string encoding
    let id_bytes: Box<dyn Buffer> = Box::new(enc.decode(id)?);

    let mut loc_hash = crypto.buf_new_insecure(16);

    // hash so it is more evenly distributed than the public key
    crypto.generic_hash(&mut loc_hash, &id_bytes, None)?;

    // this xor step may not be strictly necessary given a good distribution
    // of bytes in the generic hash (blake2b)
    // but the operation is trivial and hedges bets against minute distribution
    // differences at the individual bit level
    let mut loc: [u8; 4] = [0; 4];
    loc.clone_from_slice(&loc_hash[0..4]);
    for i in (4..16).step_by(4) {
        loc[0] ^= loc_hash[i];
        loc[1] ^= loc_hash[i + 1];
        loc[2] ^= loc_hash[i + 2];
        loc[3] ^= loc_hash[i + 3];
    }

    // finally interpret our 4 output bytes as a little-endian u32
    Ok(u32::from_le_bytes(loc).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lib3h_sodium::SodiumCryptoSystem;

    #[test]
    fn it_should_calc_location() {
        let crypto: Box<dyn CryptoSystem> =
            Box::new(SodiumCryptoSystem::new().set_pwhash_interactive());
        let location = calc_location_for_id(
            &crypto,
            "HcSciDds5OiogymxbnHKEabQ8iavqs8dwdVaGdJW76Vp4gx47tQDfGW4OWc9w5i",
        )
        .unwrap();
        assert_eq!(167996431_u32, location);
    }
}
