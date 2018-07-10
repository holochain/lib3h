use error;

use sodiumoxide::crypto::kx::x25519blake2b as so_kx;

pub fn gen_keypair () -> error::Result<(Vec<u8>, Vec<u8>)> {
    let (key_pub, key_priv) = so_kx::gen_keypair();
    Ok((key_pub.0.to_vec(), key_priv.0.to_vec()))
}

pub fn derive_client (cli_pub: &[u8], cli_priv: &[u8], srv_pub: &[u8]) -> error::Result<(Vec<u8>, Vec<u8>)> {
    let cli_pub = match so_kx::PublicKey::from_slice(cli_pub) {
        Some(v) => v,
        None => return Err(error::Error::str_error("invalid client pubkey")),
    };
    let cli_priv = match so_kx::SecretKey::from_slice(cli_priv) {
        Some(v) => v,
        None => return Err(error::Error::str_error("invalid client privkey")),
    };
    let srv_pub = match so_kx::PublicKey::from_slice(srv_pub) {
        Some(v) => v,
        None => return Err(error::Error::str_error("invalid server pubkey")),
    };
    let (recv, send) = match so_kx::client_session_keys(&cli_pub, &cli_priv, &srv_pub) {
        Ok(v) => v,
        _ => return Err(error::Error::str_error("failed generating session keys")),
    };
    Ok((recv.0.to_vec(), send.0.to_vec()))
}

pub fn derive_server (srv_pub: &[u8], srv_priv: &[u8], cli_pub: &[u8]) -> error::Result<(Vec<u8>, Vec<u8>)> {
    let srv_pub = match so_kx::PublicKey::from_slice(srv_pub) {
        Some(v) => v,
        None => return Err(error::Error::str_error("invalid server pubkey")),
    };
    let srv_priv = match so_kx::SecretKey::from_slice(srv_priv) {
        Some(v) => v,
        None => return Err(error::Error::str_error("invalid server privkey")),
    };
    let cli_pub = match so_kx::PublicKey::from_slice(cli_pub) {
        Some(v) => v,
        None => return Err(error::Error::str_error("invalid client pubkey")),
    };
    let (recv, send) = match so_kx::server_session_keys(&srv_pub, &srv_priv, &cli_pub) {
        Ok(v) => v,
        _ => return Err(error::Error::str_error("failed generating session keys")),
    };
    Ok((recv.0.to_vec(), send.0.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_derive_session_keys() {
        let (cli_pub, cli_priv) = gen_keypair().unwrap();
        let (srv_pub, srv_priv) = gen_keypair().unwrap();

        let (cli_recv, cli_send) = derive_client(&cli_pub, &cli_priv, &srv_pub).unwrap();
        let (srv_recv, srv_send) = derive_server(&srv_pub, &srv_priv, &cli_pub).unwrap();

        assert_eq!(cli_recv, srv_send);
        assert_eq!(cli_send, srv_recv);
    }
}
