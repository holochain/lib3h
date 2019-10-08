use crate::{Buffer, CryptoResult};

/// syntactic sugar for named parameters to clarify buffer usage
/// # Example
///
/// ```compile_fail
/// kx_client_session_keys!(self.crypto =>
///     client_rx: &mut c_rx,
///     client_tx: &mut c_tx,
///     client_pk: &c_pk,
///     client_sk: &c_sk,
///     server_pk: &s_pk,
/// ).unwrap();
/// ```
#[macro_export]
macro_rules! kx_client_session_keys {
    ($cs:expr => client_rx: $c_rx:expr, client_tx: $c_tx:expr, client_pk: $c_pk:expr, client_sk: $c_sk:expr, server_pk: $s_pk:expr) => {
        $cs.kx_client_session_keys($c_rx, $c_tx, $c_pk, $c_sk, $s_pk)
    };
    ($cs:expr => client_rx: $c_rx:expr, client_tx: $c_tx:expr, client_pk: $c_pk:expr, client_sk: $c_sk:expr, server_pk: $s_pk:expr,) => {
        $cs.kx_client_session_keys($c_rx, $c_tx, $c_pk, $c_sk, $s_pk)
    };
}

/// syntactic sugar for named parameters to clarify buffer usage
/// # Example
///
/// ```compile_fail
/// kx_server_session_keys!(self.crypto =>
///     server_rx: &mut s_rx,
///     server_tx: &mut s_tx,
///     server_pk: &s_pk,
///     server_sk: &s_sk,
///     client_pk: &c_pk,
/// ).unwrap();
/// ```
#[macro_export]
macro_rules! kx_server_session_keys {
    ($cs:expr => server_rx: $s_rx:expr, server_tx: $s_tx:expr, server_pk: $s_pk:expr, server_sk: $s_sk:expr, client_pk: $c_pk:expr) => {
        $cs.kx_server_session_keys($s_rx, $s_tx, $s_pk, $s_sk, $c_pk)
    };
    ($cs:expr => server_rx: $s_rx:expr, server_tx: $s_tx:expr, server_pk: $s_pk:expr, server_sk: $s_sk:expr, client_pk: $c_pk:expr,) => {
        $cs.kx_server_session_keys($s_rx, $s_tx, $s_pk, $s_sk, $c_pk)
    };
}

/// syntactic sugar for named parameters to clarify buffer usage
/// # Example
///
/// ```compile_fail
/// aead_encrypt!(self.crypto =>
///     cipher: &mut cipher,
///     message: &message,
///     adata: Some(&adata),
///     nonce: &nonce,
///     secret: &secret,
/// ).unwrap();
/// ```
#[macro_export]
macro_rules! aead_encrypt {
    ($cs:expr => cipher: $c:expr, message: $m:expr, adata: $a:expr, nonce: $n:expr, secret: $s:expr) => {
        $cs.aead_encrypt($c, $m, $a, $n, $s)
    };
    ($cs:expr => cipher: $c:expr, message: $m:expr, adata: $a:expr, nonce: $n:expr, secret: $s:expr,) => {
        $cs.aead_encrypt($c, $m, $a, $n, $s)
    };
}

/// syntactic sugar for named parameters to clarify buffer usage
/// # Example
///
/// ```compile_fail
/// aead_decrypt!(self.crypto =>
///     message: &mut msg_out,
///     cipher: &cipher,
///     adata: Some(&adata),
///     nonce: &nonce,
///     secret: &secret,
/// ).unwrap();
/// ```
#[macro_export]
macro_rules! aead_decrypt {
    ($cs:expr => message: $m:expr, cipher: $c:expr, adata: $a:expr, nonce: $n:expr, secret: $s:expr) => {
        $cs.aead_decrypt($m, $c, $a, $n, $s)
    };
    ($cs:expr => message: $m:expr, cipher: $c:expr, adata: $a:expr, nonce: $n:expr, secret: $s:expr,) => {
        $cs.aead_decrypt($m, $c, $a, $n, $s)
    };
}

/// A trait describing a cryptographic system implementation compatible
/// with Lib3h and Holochain.
#[allow(clippy::borrowed_box)]
pub trait CryptoSystem: Sync {
    /// Crypto System is designed to be used as a trait-object
    /// Since we can't get a sized clone, provide clone in a Box.
    fn box_clone(&self) -> Box<dyn CryptoSystem>;

    /// helps work around some sizing issues with rust trait-objects
    fn as_crypto_system(&self) -> &dyn CryptoSystem;

    /// create a new memory secured buffer
    /// that is compatible with this crypto system
    fn buf_new_secure(&self, size: usize) -> Box<dyn Buffer>;

    /// this is just a helper to create a
    /// sized boxed Vec<u8> as a Box<dyn Buffer>
    fn buf_new_insecure(&self, size: usize) -> Box<dyn Buffer> {
        Box::new(vec![0; size])
    }

    // -- random methods -- //

    /// fill all the bytes in the buffer with secure random data
    fn randombytes_buf(&self, buffer: &mut Box<dyn Buffer>) -> CryptoResult<()>;

    // -- derivation methods -- //

    /// bytelength of sha256 hash
    fn hash_sha256_bytes(&self) -> usize;

    /// bytelength of sha512 hash
    fn hash_sha512_bytes(&self) -> usize;

    /// compute a sha256 hash for `data`, storing it in `hash`
    fn hash_sha256(&self, hash: &mut Box<dyn Buffer>, data: &Box<dyn Buffer>) -> CryptoResult<()>;

    /// compute a sha512 hash for `data`, storing it in `hash`
    fn hash_sha512(&self, hash: &mut Box<dyn Buffer>, data: &Box<dyn Buffer>) -> CryptoResult<()>;

    /// min bytelength of generic hash output
    fn generic_hash_min_bytes(&self) -> usize;

    /// max bytelength of generic hash output
    fn generic_hash_max_bytes(&self) -> usize;

    /// min bytelength of generic hash key
    fn generic_hash_key_min_bytes(&self) -> usize;

    /// max bytelength of generic hash key
    fn generic_hash_key_max_bytes(&self) -> usize;

    /// compute a deterministic (BLAKE2b) generic hash for given data
    /// key can be `None`
    fn generic_hash(
        &self,
        hash: &mut Box<dyn Buffer>,
        data: &Box<dyn Buffer>,
        key: Option<&Box<dyn Buffer>>,
    ) -> CryptoResult<()>;

    /// bytelength of pwhash salt
    fn pwhash_salt_bytes(&self) -> usize;

    /// bytelength of pwhash
    fn pwhash_bytes(&self) -> usize;

    /// run a cpu/memory intensive password hash against password / salt
    fn pwhash(
        &self,
        hash: &mut Box<dyn Buffer>,
        password: &Box<dyn Buffer>,
        salt: &Box<dyn Buffer>,
    ) -> CryptoResult<()>;

    /// bytelength of parent key from which to derive
    fn kdf_key_bytes(&self) -> usize;

    /// bytelength of key derivation context
    fn kdf_context_bytes(&self) -> usize;

    /// minimum bytelength of key derivation buffers
    fn kdf_min_bytes(&self) -> usize;

    /// maximum bytelength of key derivation buffers
    fn kdf_max_bytes(&self) -> usize;

    /// derive a new deterministic key based of index, context, and parent
    fn kdf(
        &self,
        out_buffer: &mut Box<dyn Buffer>,
        index: u64,
        context: &Box<dyn Buffer>,
        parent: &Box<dyn Buffer>,
    ) -> CryptoResult<()>;

    // -- signature methods -- //

    /// bytelength of signature seed
    fn sign_seed_bytes(&self) -> usize;

    /// bytelength of signature public key
    fn sign_public_key_bytes(&self) -> usize;

    /// bytelength of signature secret key
    fn sign_secret_key_bytes(&self) -> usize;

    /// bytelength of a digital signature
    fn sign_bytes(&self) -> usize;

    /// generate a deterministic signature public / secret keypair
    /// based off the given seed entropy
    fn sign_seed_keypair(
        &self,
        seed: &Box<dyn Buffer>,
        public_key: &mut Box<dyn Buffer>,
        secret_key: &mut Box<dyn Buffer>,
    ) -> CryptoResult<()>;

    /// generate a pure entropy based signature public / secret keypair
    fn sign_keypair(
        &self,
        public_key: &mut Box<dyn Buffer>,
        secret_key: &mut Box<dyn Buffer>,
    ) -> CryptoResult<()>;

    /// generate a digital signature for `message` with the given secret key
    fn sign(
        &self,
        signature: &mut Box<dyn Buffer>,
        message: &Box<dyn Buffer>,
        secret_key: &Box<dyn Buffer>,
    ) -> CryptoResult<()>;

    /// verify that the digital `signature` is valid for given `message` and
    /// `public_key`
    fn sign_verify(
        &self,
        signature: &Box<dyn Buffer>,
        message: &Box<dyn Buffer>,
        public_key: &Box<dyn Buffer>,
    ) -> CryptoResult<bool>;

    // -- key exchange methods -- //

    /// bytelength of key exchange seed
    fn kx_seed_bytes(&self) -> usize;

    /// bytelength of key exchange public key
    fn kx_public_key_bytes(&self) -> usize;

    /// bytelength of key exchange secret key
    fn kx_secret_key_bytes(&self) -> usize;

    /// bytelength of session keys derived from key exchange
    fn kx_session_key_bytes(&self) -> usize;

    /// generate a deterministic key exchange public / secret keypair
    /// based off the given seed entropy
    fn kx_seed_keypair(
        &self,
        seed: &Box<dyn Buffer>,
        public_key: &mut Box<dyn Buffer>,
        secret_key: &mut Box<dyn Buffer>,
    ) -> CryptoResult<()>;

    /// generate a pure entropy based key exchange public / secret keypair
    fn kx_keypair(
        &self,
        public_key: &mut Box<dyn Buffer>,
        secret_key: &mut Box<dyn Buffer>,
    ) -> CryptoResult<()>;

    /// generate key exchange session keys from "client" perspective
    /// for named arguments for code clarity, consider using the macro:
    ///   kx_client_session_keys!
    fn kx_client_session_keys(
        &self,
        client_rx: &mut Box<dyn Buffer>,
        client_tx: &mut Box<dyn Buffer>,
        client_pk: &Box<dyn Buffer>,
        client_sk: &Box<dyn Buffer>,
        server_pk: &Box<dyn Buffer>,
    ) -> CryptoResult<()>;

    /// generate key exchange session keys from "server" perspective
    /// for named arguments for code clarity, consider using the macro:
    ///   kx_server_session_keys!
    fn kx_server_session_keys(
        &self,
        server_rx: &mut Box<dyn Buffer>,
        server_tx: &mut Box<dyn Buffer>,
        server_pk: &Box<dyn Buffer>,
        server_sk: &Box<dyn Buffer>,
        client_pk: &Box<dyn Buffer>,
    ) -> CryptoResult<()>;

    // -- aead encryption methods -- //

    /// bytelength of key exchange seed
    fn aead_nonce_bytes(&self) -> usize;

    /// bytelength of aead authentication tag
    fn aead_auth_bytes(&self) -> usize;

    /// bytelength of aead symmetric key
    fn aead_secret_bytes(&self) -> usize;

    /// encrypt `message` into buffer `cipher`
    /// for named arguments for code clarity, consider using the macro:
    ///   aead_encrypt!
    fn aead_encrypt(
        &self,
        cipher: &mut Box<dyn Buffer>,
        message: &Box<dyn Buffer>,
        adata: Option<&Box<dyn Buffer>>,
        nonce: &Box<dyn Buffer>,
        secret: &Box<dyn Buffer>,
    ) -> CryptoResult<()>;

    /// decrypt `cipher` into buffer `message`
    /// for named arguments for code clarity, consider using the macro:
    ///   aead_encrypt!
    fn aead_decrypt(
        &self,
        message: &mut Box<dyn Buffer>,
        cipher: &Box<dyn Buffer>,
        adata: Option<&Box<dyn Buffer>>,
        nonce: &Box<dyn Buffer>,
        secret: &Box<dyn Buffer>,
    ) -> CryptoResult<()>;
}

pub mod crypto_system_test;
