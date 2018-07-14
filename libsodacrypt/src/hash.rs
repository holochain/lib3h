/*!
Hashing utility functions.
*/

use error;

use sodiumoxide::crypto::hash::sha256::hash as so_sha256;
use sodiumoxide::crypto::hash::sha512::hash as so_sha512;

/**
Given `data`, produce a sha256 sum.

# Examples

```
use libsodacrypt::hash::sha256;

let expects: Vec<u8> = vec![44, 242, 77, 186, 95, 176, 163, 14, 38, 232, 59, 42, 197, 185, 226, 158, 27, 22, 30, 92, 31, 167, 66, 94, 115, 4, 51, 98, 147, 139, 152, 36];
assert_eq!(expects, sha256(b"hello").unwrap());
```
*/
pub fn sha256 (data: &[u8]) -> error::Result<Vec<u8>> {
    Ok(so_sha256(data).0.to_vec())
}

/**
Given `data`, produce a sha512 sum.

# Examples

```
use libsodacrypt::hash::sha512;

let expects: Vec<u8> = vec![155, 113, 210, 36, 189, 98, 243, 120, 93, 150, 212, 106, 211, 234, 61, 115, 49, 155, 251, 194, 137, 12, 170, 218, 226, 223, 247, 37, 25, 103, 60, 167, 35, 35, 195, 217, 155, 165, 193, 29, 124, 122, 204, 110, 20, 184, 197, 218, 12, 70, 99, 71, 92, 46, 92, 58, 222, 244, 111, 115, 188, 222, 192, 67];
assert_eq!(expects, sha512(b"hello").unwrap());
```
*/
pub fn sha512 (data: &[u8]) -> error::Result<Vec<u8>> {
    Ok(so_sha512(data).0.to_vec())
}
