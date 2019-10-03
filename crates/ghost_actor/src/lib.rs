#![recursion_limit = "128"]
extern crate crossbeam_channel;
extern crate inflector;
//#[macro_use]
extern crate lazy_static;
extern crate lib3h_zombie_actor;
extern crate proc_macro2;
//#[macro_use]
extern crate syn;
#[allow(unused_imports)]
#[macro_use]
extern crate quote;

use std::sync::{Arc, Mutex, MutexGuard};

pub use lib3h_zombie_actor::{ErrorKind as GhostErrorKind, GhostError, GhostResult};

fn ghost_try_lock<'a, M>(m: &'a mut Arc<Mutex<M>>) -> MutexGuard<'a, M> {
    let mut wait_ms = 0;
    for _ in 0..100 {
        match m.try_lock() {
            Ok(g) => return g,
            Err(_) => {
                std::thread::sleep(std::time::Duration::from_millis(wait_ms));
                wait_ms += 1;
            }
        }
    }
    panic!("failed to obtain mutex lock");
}

mod ghost_protocol;
pub use ghost_protocol::*;

mod ghost_actor;
pub use crate::ghost_actor::*;

mod ghost_system;
pub use ghost_system::*;

pub mod code_gen;

pub mod prelude {
    pub use super::*;
}
