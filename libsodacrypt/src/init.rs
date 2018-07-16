use errors::*;
use sodiumoxide;

lazy_static! {
    static ref SO_INIT: ::std::result::Result<(), ()> = sodiumoxide::init();
}

pub fn check () -> Result<()> {
    match *SO_INIT {
        Err(_) => Err(ErrorKind::FailedLibSodiumInit.into()),
        Ok(_) => Ok(()),
    }
}
