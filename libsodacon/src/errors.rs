/*!
error types for libsodacon
*/

use hex;
use libsodacrypt;
use rmp_serde;

error_chain! {
    links {
        SodaCrypt(libsodacrypt::errors::Error, libsodacrypt::errors::ErrorKind);
    }

    foreign_links {
        RmpDecode(rmp_serde::decode::Error);
        RmpEncode(rmp_serde::encode::Error);
        Io(::std::io::Error);
        AddrParseError(::std::net::AddrParseError);
        FromHexError(hex::FromHexError);
    }

    errors {
    }
}
