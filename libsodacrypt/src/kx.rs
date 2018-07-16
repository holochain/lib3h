/*!
Key exchange utility functions.

# Examples

```
use libsodacrypt::kx::*;

let (cli_pub, cli_priv) = gen_keypair().unwrap();
let (srv_pub, srv_priv) = gen_keypair().unwrap();

// client / server need to exchange public keys

let (cli_recv, cli_send) = derive_client(&cli_pub, &cli_priv, &srv_pub).unwrap();
let (srv_recv, srv_send) = derive_server(&srv_pub, &srv_priv, &cli_pub).unwrap();

assert_eq!(cli_recv, srv_send);
assert_eq!(cli_send, srv_recv);
```
*/

use errors::*;
use init;

use sodiumoxide::crypto::kx::x25519blake2b as so_kx;

/**
Generate a key exchange keypair.

# Examples

```
use libsodacrypt::kx::gen_keypair;

let (key_pub, key_priv) = gen_keypair().unwrap();
```
*/
pub fn gen_keypair() -> Result<(Vec<u8>, Vec<u8>)> {
    init::check()?;
    let (key_pub, key_priv) = so_kx::gen_keypair();
    Ok((key_pub.0.to_vec(), key_priv.0.to_vec()))
}

/**
Client-side derivation of secret symmetric keys.

# Examples

```
use libsodacrypt::kx::*;

let (cli_pub, cli_priv) = gen_keypair().unwrap();

// obtain the server's public key
let (srv_pub, _ignore) = gen_keypair().unwrap();

// derive the secret symmetric keys.
let (cli_recv, cli_send) = derive_client(&cli_pub, &cli_priv, &srv_pub).unwrap();
```
*/
pub fn derive_client(
    cli_pub: &[u8],
    cli_priv: &[u8],
    srv_pub: &[u8],
) -> Result<(Vec<u8>, Vec<u8>)> {
    init::check()?;
    let cli_pub = match so_kx::PublicKey::from_slice(cli_pub) {
        Some(v) => v,
        None => return Err(ErrorKind::InvalidClientPubKey.into()),
    };
    let cli_priv = match so_kx::SecretKey::from_slice(cli_priv) {
        Some(v) => v,
        None => return Err(ErrorKind::InvalidClientPrivKey.into()),
    };
    let srv_pub = match so_kx::PublicKey::from_slice(srv_pub) {
        Some(v) => v,
        None => return Err(ErrorKind::InvalidServerPubKey.into()),
    };
    let (recv, send) = match so_kx::client_session_keys(&cli_pub, &cli_priv, &srv_pub) {
        Ok(v) => v,
        _ => return Err("failed generating session keys".into()),
    };
    Ok((recv.0.to_vec(), send.0.to_vec()))
}

/**
Server-side derivation of secret symmetric keys.

# Examples

```
use libsodacrypt::kx::*;

let (srv_pub, srv_priv) = gen_keypair().unwrap();

// obtain the client's public key
let (cli_pub, _ignore) = gen_keypair().unwrap();

// derive the secret symmetric keys.
let (srv_recv, srv_send) = derive_server(&srv_pub, &srv_priv, &cli_pub).unwrap();
```
*/
pub fn derive_server(
    srv_pub: &[u8],
    srv_priv: &[u8],
    cli_pub: &[u8],
) -> Result<(Vec<u8>, Vec<u8>)> {
    init::check()?;
    let srv_pub = match so_kx::PublicKey::from_slice(srv_pub) {
        Some(v) => v,
        None => return Err(ErrorKind::InvalidServerPubKey.into()),
    };
    let srv_priv = match so_kx::SecretKey::from_slice(srv_priv) {
        Some(v) => v,
        None => return Err(ErrorKind::InvalidServerPrivKey.into()),
    };
    let cli_pub = match so_kx::PublicKey::from_slice(cli_pub) {
        Some(v) => v,
        None => return Err(ErrorKind::InvalidClientPubKey.into()),
    };
    let (recv, send) = match so_kx::server_session_keys(&srv_pub, &srv_priv, &cli_pub) {
        Ok(v) => v,
        _ => return Err("failed generating session keys".into()),
    };
    Ok((recv.0.to_vec(), send.0.to_vec()))
}
