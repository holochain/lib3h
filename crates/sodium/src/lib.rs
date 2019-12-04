#![warn(unused_extern_crates)]
#![allow(warnings)]
extern crate rust_sodium_sys;
#[macro_use]
extern crate lazy_static;
extern crate lib3h_crypto_api;

thread_local! {
    pub static DID_INIT: bool = {
        unsafe {
            rust_sodium_sys::sodium_init();
        }
        true
    };
}

/// make sure sodium_init is called
pub fn check_init() {
    DID_INIT.with(|i| {
        assert_eq!(true, *i);
    });
}

/// make invoking ffi functions taking SecBuf references more readable
macro_rules! raw_ptr_void {
    ($name: ident) => {
        $name.as_mut_ptr() as *mut libc::c_void
    };
}

/// make invoking ffi functions taking SecBuf references more readable
macro_rules! raw_ptr_char {
    ($name: ident) => {
        $name.as_mut_ptr() as *mut libc::c_uchar
    };
}

/// make invoking ffi functions taking SecBuf references more readable
macro_rules! raw_ptr_char_immut {
    ($name: ident) => {
        $name.as_ptr() as *const libc::c_uchar
    };
}

/// make invoking ffi functions taking SecBuf references more readable
macro_rules! raw_ptr_ichar_immut {
    ($name: ident) => {
        $name.as_ptr() as *const libc::c_char
    };
}

pub mod aead;
pub mod hash;
pub mod kdf;
pub mod kx;
pub mod pwhash;
pub mod secbuf;
pub mod secbuf_random;
pub mod secbuf_util;
pub mod sign;

mod crypto_system;
pub use crypto_system::{SecureBuffer, SodiumCryptoSystem};
