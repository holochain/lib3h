/*!
error types for lib3h
*/

use libsodacon;
use rmp_serde;

error_chain! {
    links {
        SodaCon(libsodacon::errors::Error, libsodacon::errors::ErrorKind);
    }

    foreign_links {
        RmpDecode(rmp_serde::decode::Error);
        RmpEncode(rmp_serde::encode::Error);
    }

    errors {
    }
}
