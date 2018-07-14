/*!
Cryptographically secure, thread-safe, random bytes.
*/

use error;

use sodiumoxide::randombytes::randombytes as so_bytes;

/**
Produce `count` random bytes.

# Examples

```
use libsodacrypt::rand::rand_bytes;

let bytes = rand_bytes(12).unwrap();
assert_eq!(12, bytes.len());
```
*/
pub fn rand_bytes (count: usize) -> error::Result<Vec<u8>> {
    Ok(so_bytes(count))
}
