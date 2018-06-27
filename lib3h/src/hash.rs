use openssl;
use tiny_keccak;

pub fn hash(data: &[u8]) -> Result<Vec<u8>, openssl::error::ErrorStack> {
    let data = tiny_keccak::sha3_512(data);
    let mut hash = openssl::sha::Sha256::new();
    hash.update(&data);
    Ok(hash.finish().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex;

    #[test]
    fn hash_test() {
        let test = String::from("hello");
        let res = hash(test.as_bytes()).unwrap();
        let res = hex::encode(&res);

        assert_eq!(
            res,
            String::from("458ad4eb514359d2142d2cd7a5ebfd37b3bc244e8f9e7ddab270e4065936c1ca")
        );
    }
}
