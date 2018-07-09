use error;

use sodiumoxide::randombytes::randombytes as so_bytes;

pub fn rand_bytes (count: usize) -> error::Result<Vec<u8>> {
    Ok(so_bytes(count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_generates_correct_length() {
        let bytes = rand_bytes(12).unwrap();
        assert_eq!(12, bytes.len());
    }
}
