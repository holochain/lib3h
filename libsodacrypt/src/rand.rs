/*!
Cryptographically secure, thread-safe, random bytes.
*/

use errors::*;
use init;

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
pub fn rand_bytes(count: usize) -> Result<Vec<u8>> {
    init::check()?;
    Ok(so_bytes(count))
}
