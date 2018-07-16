/*!
All libsodacrypt apis will return an errors::Result.
*/

error_chain! {
    links {
    }

    foreign_links {
        Io(::std::io::Error);
    }

    errors {
        InvalidPubKey
        InvalidPrivKey
        InvalidSeed
        InvalidSignature
        InvalidNonce
        InvalidPresharedKey
        InvalidClientPubKey
        InvalidClientPrivKey
        InvalidServerPubKey
        InvalidServerPrivKey
        FailedToDecrypt
    }
}
