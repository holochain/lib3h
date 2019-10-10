extern crate lock_api;
extern crate parking_lot;

use std::{
    borrow::{Borrow, BorrowMut},
    convert::{AsMut, AsRef},
    ops::{Deref, DerefMut},
};

#[must_use]
pub struct GhostMutexGuard<'lt, T: ?Sized + 'lt> {
    guard: Option<parking_lot::MutexGuard<'lt, T>>,
}

impl<'lt, T: ?Sized + 'lt> Drop for GhostMutexGuard<'lt, T> {
    fn drop(&mut self) {
        match std::mem::replace(&mut self.guard, None) {
            Some(guard) => {
                parking_lot::MutexGuard::unlock_fair(guard);
            }
            None => (),
        }
    }
}

impl<'lt, T: ?Sized + 'lt> Deref for GhostMutexGuard<'lt, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &*self.guard.as_ref().expect("exists")
    }
}

impl<'lt, T: ?Sized + 'lt> DerefMut for GhostMutexGuard<'lt, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.guard.as_mut().expect("exists")
    }
}

impl<'lt, T: ?Sized + 'lt> AsRef<T> for GhostMutexGuard<'lt, T> {
    fn as_ref(&self) -> &T {
        self.deref()
    }
}

impl<'lt, T: ?Sized + 'lt> AsMut<T> for GhostMutexGuard<'lt, T> {
    fn as_mut(&mut self) -> &mut T {
        self.deref_mut()
    }
}

impl<'lt, T: ?Sized + 'lt> Borrow<T> for GhostMutexGuard<'lt, T> {
    fn borrow(&self) -> &T {
        self.deref()
    }
}

impl<'lt, T: ?Sized + 'lt> BorrowMut<T> for GhostMutexGuard<'lt, T> {
    fn borrow_mut(&mut self) -> &mut T {
        self.deref_mut()
    }
}

impl<'lt, T: ?Sized + 'lt + std::fmt::Debug> std::fmt::Debug for GhostMutexGuard<'lt, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "{:#?}", self.deref())
        } else {
            write!(f, "{:?}", self.deref())
        }
    }
}

pub struct GhostMutex<T: ?Sized> {
    mutex: parking_lot::Mutex<T>,
}

impl<T> GhostMutex<T> {
    pub fn new(t: T) -> Self {
        Self {
            mutex: parking_lot::Mutex::new(t),
        }
    }

    pub fn into_inner(self) -> T {
        self.mutex.into_inner()
    }
}

impl<T: ?Sized> GhostMutex<T> {
    pub fn lock(&self) -> GhostMutexGuard<'_, T> {
        match self
            .mutex
            .try_lock_for(std::time::Duration::from_millis(10))
        {
            None => panic!("failed to obtain lock within timeout"),
            Some(g) => GhostMutexGuard { guard: Some(g) },
        }
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.mutex.get_mut()
    }
}

impl<T> From<T> for GhostMutex<T> {
    fn from(t: T) -> Self {
        GhostMutex::new(t)
    }
}

impl<T: ?Sized + Default> Default for GhostMutex<T> {
    fn default() -> Self {
        GhostMutex::new(Default::default())
    }
}

impl<T: ?Sized + std::fmt::Debug> std::fmt::Debug for GhostMutex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut f = f.debug_struct("GhostMutex");
        let f = match self.mutex.try_lock() {
            Some(guard) => f.field("inner", &&*guard),
            None => f.field("inner", &"<locked>".to_string()),
        };

        f.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn it_should_debug() {
        let a = GhostMutex::new(true);
        assert_eq!("GhostMutex { inner: true }", &format!("{:?}", a));
    }

    #[test]
    fn it_should_be_fair_ish() {
        let tally: Vec<u64> = vec![0, 0, 0, 0, 0];
        let tally_count = tally.len();
        let tally = Arc::new(GhostMutex::new(tally));

        let cont = Arc::new(GhostMutex::new(true));

        let mut threads = Vec::new();

        for i in 0..tally_count {
            let my_tally = tally.clone();
            let my_cont = cont.clone();
            threads.push(std::thread::spawn(move || loop {
                {
                    let mut my_tally = my_tally.lock();
                    let my_tally = &mut *my_tally;
                    *my_tally.get_mut(i).unwrap() += 1;
                }
                {
                    if !*my_cont.lock() {
                        break;
                    }
                }
                std::thread::sleep(std::time::Duration::from_micros(1));
            }));
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
        *cont.lock() = false;

        for t in threads.drain(..) {
            t.join().unwrap();
        }

        let tally = Arc::try_unwrap(tally).unwrap().into_inner();
        let mut max = *tally.get(0).unwrap();
        let mut min = *tally.get(0).unwrap();
        for t in tally.iter() {
            if *t > max {
                max = *t;
            }
            if *t < min {
                min = *t;
            }
        }

        let min_pct = min * 100 / max;

        println!(
            "got: min {} max {} min % {} out of {:?}",
            min, max, min_pct, tally
        );
        // make sure our min is at least 10% of our max
        assert!(min_pct > 10);
    }
}
