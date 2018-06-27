use error;
use hex;
use num_bigint;
//use num_traits;

use num_traits::ToPrimitive;

pub fn u32_tag_for_hash(hash: &[u8]) -> error::Result<u32> {
    let hex = hex::encode(hash);

    let mut big = match num_bigint::BigUint::parse_bytes(hex.as_bytes(), 16) {
        Some(v) => v,
        None => return Err(error::Error::from("hash parse error")),
    };

    big %= <u32>::max_value();

    Ok(match big.to_u32() {
        Some(v) => v,
        None => return Err(error::Error::from("hash parse error")),
    })
}
