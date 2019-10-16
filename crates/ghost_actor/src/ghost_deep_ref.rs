use crate::*;
use std::sync::{Arc, Weak};

/// will be invoked when the internal ref is updated
pub(crate) type DeepRefSetCb<'lt, X> =
    Box<dyn FnMut(Weak<GhostMutex<X>>) -> GhostResult<bool> + 'lt>;

/// internal deep ref data
struct DeepRefInner<'lt, X: 'lt + Send + Sync> {
    p: std::marker::PhantomData<&'lt i8>,
    r: Weak<GhostMutex<X>>,
    callbacks: Vec<DeepRefSetCb<'lt, X>>,
}

impl<'lt, X: 'lt + Send + Sync> DeepRefInner<'lt, X> {
    pub fn new() -> Self {
        Self {
            p: std::marker::PhantomData,
            r: Weak::new(),
            callbacks: Vec::new(),
        }
    }

    pub fn set(&mut self, user_data: Weak<GhostMutex<X>>) -> GhostResult<()> {
        std::mem::replace(&mut self.r, user_data.clone());
        for mut cb in self.callbacks.drain(..).collect::<Vec<_>>() {
            match cb(user_data.clone()) {
                Err(e) => panic!("{:?}", e),
                Ok(keep) => {
                    if keep {
                        self.callbacks.push(cb);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn push_cb(&mut self, mut cb: DeepRefSetCb<'lt, X>) -> GhostResult<()> {
        if !cb(self.r.clone())? {
            return Ok(());
        }
        self.callbacks.push(cb);
        Ok(())
    }
}

impl<'lt, X: 'lt + Send + Sync> std::ops::Deref for DeepRefInner<'lt, X> {
    type Target = Weak<GhostMutex<X>>;

    fn deref(&self) -> &Self::Target {
        &self.r
    }
}

#[allow(dead_code)]
/// helper for access to internal reference
pub(crate) struct DeepRefGuard<'a, 'lt, X: 'lt + Send + Sync> {
    guard: GhostMutexGuard<'a, DeepRefInner<'lt, X>>,
}

impl<'a, 'lt, X: 'lt + Send + Sync> std::ops::Deref for DeepRefGuard<'a, 'lt, X> {
    type Target = Weak<GhostMutex<X>>;

    fn deref(&self) -> &Self::Target {
        &*self.guard
    }
}

/// private deep reference
pub(crate) struct DeepRef<'lt, X: 'lt + Send + Sync> {
    p: std::marker::PhantomData<&'lt i8>,
    r: Arc<GhostMutex<DeepRefInner<'lt, X>>>,
}

unsafe impl<'lt, X: 'lt + Send + Sync> Send for DeepRef<'lt, X> {}
unsafe impl<'lt, X: 'lt + Send + Sync> Sync for DeepRef<'lt, X> {}

impl<'lt, X: 'lt + Send + Sync> Clone for DeepRef<'lt, X> {
    fn clone(&self) -> Self {
        Self {
            p: std::marker::PhantomData,
            r: self.r.clone(),
        }
    }
}

impl<'lt, X: 'lt + Send + Sync> DeepRef<'lt, X> {
    pub fn new() -> Self {
        Self {
            p: std::marker::PhantomData,
            r: Arc::new(GhostMutex::new(DeepRefInner::new())),
        }
    }

    pub fn set(&mut self, user_data: Weak<GhostMutex<X>>) -> GhostResult<()> {
        self.r.lock().set(user_data)
    }

    pub fn push_cb(&mut self, cb: DeepRefSetCb<'lt, X>) -> GhostResult<()> {
        self.r.lock().push_cb(cb)
    }

    #[allow(dead_code)]
    pub fn lock<'a>(&'a self) -> DeepRefGuard<'a, 'lt, X> {
        DeepRefGuard {
            guard: self.r.lock(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_it_send_sync<DR: Send + Sync>(_dr: DR) {}

    #[test]
    fn it_should_be_send_and_sync() {
        let dr: DeepRef<'_, i32> = DeepRef::new();
        is_it_send_sync(dr);
    }

    #[test]
    fn it_can_deep_ref() {
        let mut r: DeepRef<'_, i32> = DeepRef::new();

        {
            if let Some(_) = r.lock().upgrade() {
                panic!("should be un-set to start");
            };
        }

        // make sure we can execute callbacks when user_data is updated
        r.push_cb(Box::new(|user_data| {
            if let Some(data) = user_data.upgrade() {
                *data.lock() = 42;
            }
            Ok(true)
        }))
        .unwrap();

        // callback should re-set this to 42
        let val = Arc::new(GhostMutex::new(0));
        r.set(Arc::downgrade(&val)).unwrap();

        {
            match r.lock().upgrade() {
                Some(v) => {
                    assert_eq!(42, *v.lock());
                }
                None => panic!("should be set"),
            };
        }

        // verify that we have mutex access to the core data
        *val.lock() = 3;

        {
            match r.lock().upgrade() {
                Some(v) => {
                    assert_eq!(3, *v.lock());
                }
                None => panic!("should be set"),
            };
        }
    }
}
