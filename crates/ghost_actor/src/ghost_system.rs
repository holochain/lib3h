use crate::*;
use holochain_tracing::*;
use std::sync::{Arc, Weak};

/// struct used for hinting on whether / when to next run this process fn
pub struct GhostProcessInstructions {
    should_continue: bool,
    next_run_delay_ms: u64,
}

impl Default for GhostProcessInstructions {
    fn default() -> Self {
        Self {
            should_continue: false,
            next_run_delay_ms: 0,
        }
    }
}

pub trait GhostSystem<'lt, S: GhostSystemRef<'lt>> {
    /// execute all queued processor functions
    fn process(&mut self) -> GhostResult<()>;

    fn create_ref(&self) -> S;

    /// get a GhostSystemRef capable of enqueueing new processor functions
    /// without creating any deadlocks
    fn create_external_system_ref<X: 'lt + Send + Sync>(
        &self,
    ) -> (
        GhostActorSystem<'lt, X, S>,
        FinalizeExternalSystemRefCb<'lt, X>,
    ) {
        let mut deep_ref = DeepRef::new();
        let system = GhostActorSystem::new(self.create_ref(), deep_ref.clone());
        (system, Box::new(move |user_data| deep_ref.set(user_data)))
    }
}

impl GhostProcessInstructions {
    pub fn should_continue(&self) -> bool {
        self.should_continue
    }

    pub fn set_should_continue(mut self, should_continue: bool) -> Self {
        self.should_continue = should_continue;
        self
    }

    pub fn next_run_delay_ms(&self) -> u64 {
        self.next_run_delay_ms
    }

    pub fn set_next_run_delay_ms(mut self, next_run_delay_ms: u64) -> Self {
        self.next_run_delay_ms = next_run_delay_ms;
        self
    }
}

/// typedef for a periodic process callback
pub type GhostProcessCb<'lt> =
    Box<dyn FnMut() -> GhostResult<GhostProcessInstructions> + 'lt + Send + Sync>;

/// internal struct for tracking processor fns
struct GhostProcessorData<'lt> {
    pub delay_until: Option<std::time::Instant>,
    pub cb: GhostProcessCb<'lt>,
}

/// inner struct trackes queued process callbacks
struct GhostSystemInner<'lt> {
    process_recv: crossbeam_channel::Receiver<GhostProcessorData<'lt>>,
    process_queue: Vec<GhostProcessorData<'lt>>,
}

impl<'lt> GhostSystemInner<'lt> {
    /// new inner system
    fn new(process_recv: crossbeam_channel::Receiver<GhostProcessorData<'lt>>) -> Self {
        Self {
            process_recv,
            process_queue: Vec::new(),
        }
    }

    /// first, check for new process functions,
    /// then, actually loop through the queued process functions
    /// keep them for next time if they return true
    fn process(&mut self) -> GhostResult<()> {
        while let Ok(item) = self.process_recv.try_recv() {
            self.process_queue.push(item);
        }
        let mut errors = Vec::new();
        for mut item in self.process_queue.drain(..).collect::<Vec<_>>() {
            match &item.delay_until {
                Some(delay_until) if &std::time::Instant::now() < delay_until => {
                    self.process_queue.push(item)
                }
                _ => {
                    let instructions = match (item.cb)() {
                        Err(e) => {
                            errors.push(e);
                            continue;
                        }
                        Ok(i) => i,
                    };
                    if instructions.should_continue {
                        let delay_ms = instructions.next_run_delay_ms();
                        item.delay_until = match delay_ms {
                            0 => None,
                            _ => Some(
                                std::time::Instant::now()
                                    .checked_add(std::time::Duration::from_millis(delay_ms))
                                    .expect("can add ms"),
                            ),
                        };
                        self.process_queue.push(item);
                    }
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.into())
        }
    }
}
pub type FinalizeExternalSystemRefCb<'lt, X> =
    Box<dyn FnOnce(Weak<GhostMutex<X>>) -> GhostResult<()> + 'lt>;

//--------------------------------------------------------------------------------------------------
// SingleThreadedGhostSystem
//--------------------------------------------------------------------------------------------------

/// Ref that allows queueing of new process functions
/// but does not have the ability to actually run process
#[derive(Clone)]
pub struct SingleThreadedGhostSystemRef<'lt> {
    process_send: crossbeam_channel::Sender<GhostProcessorData<'lt>>,
    // just a refcount
    _system_inner: Arc<GhostMutex<GhostSystemInner<'lt>>>,
}

impl<'lt> GhostSystemRef<'lt> for SingleThreadedGhostSystemRef<'lt> {
    /// enqueue a new processor function for periodic execution
    fn enqueue_processor(
        &mut self,
        start_delay_ms: u64,
        cb: GhostProcessCb<'lt>,
    ) -> GhostResult<()> {
        let data = GhostProcessorData {
            delay_until: match start_delay_ms {
                0 => None,
                _ => Some(
                    std::time::Instant::now()
                        .checked_add(std::time::Duration::from_millis(start_delay_ms))
                        .expect("can add ms"),
                ),
            },
            cb,
        };
        self.process_send.send(data)?;
        Ok(())
    }
}

/// the main ghost system struct. Allows queueing new processor functions
/// and provides a process() function to actually execute them
pub struct SingleThreadedGhostSystem<'lt> {
    process_send: crossbeam_channel::Sender<GhostProcessorData<'lt>>,
    system_inner: Arc<GhostMutex<GhostSystemInner<'lt>>>,
}

impl<'lt> SingleThreadedGhostSystem<'lt> {
    /// create a new ghost system
    pub fn new() -> Self {
        let (process_send, process_recv) = crossbeam_channel::unbounded();
        Self {
            process_send,
            system_inner: Arc::new(GhostMutex::new(GhostSystemInner::new(process_recv))),
        }
    }
}

impl<'lt> GhostSystem<'lt, SingleThreadedGhostSystemRef<'lt>> for SingleThreadedGhostSystem<'lt> {
    /// execute all queued processor functions
    fn process(&mut self) -> GhostResult<()> {
        self.system_inner.lock().process()
    }

    fn create_ref(&self) -> SingleThreadedGhostSystemRef<'lt> {
        SingleThreadedGhostSystemRef {
            process_send: self.process_send.clone(),
            _system_inner: self.system_inner.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_start_delay() {
        #[derive(Debug)]
        struct Test {
            start_delay: bool,
            non_start_delay: bool,
        }

        let test = Arc::new(GhostMutex::new(Test {
            start_delay: false,
            non_start_delay: false,
        }));

        let mut sys = SingleThreadedGhostSystem::default();

        let test_clone = test.clone();
        sys.create_ref()
            .enqueue_processor(
                2,
                Box::new(move || {
                    test_clone.lock().start_delay = true;
                    Ok(GhostProcessInstructions::default())
                }),
            )
            .unwrap();

        let test_clone = test.clone();
        sys.create_ref()
            .enqueue_processor(
                0,
                Box::new(move || {
                    test_clone.lock().non_start_delay = true;
                    Ok(GhostProcessInstructions::default())
                }),
            )
            .unwrap();

        sys.process().unwrap();

        {
            let test = test.lock();
            println!("start_delay_result {:?}", *test);
            assert_eq!(true, test.non_start_delay);
            assert_eq!(false, test.start_delay);
        }

        std::thread::sleep(std::time::Duration::from_millis(3));
        sys.process().unwrap();

        {
            let test = test.lock();
            println!("start_delay_result {:?}", *test);
            assert_eq!(true, test.non_start_delay);
            assert_eq!(true, test.start_delay);
        }
    }

    #[test]
    fn it_should_periodic_delay() {
        #[derive(Debug)]
        struct Test {
            delayed_count: i32,
            non_delayed_count: i32,
        }

        let test = Arc::new(GhostMutex::new(Test {
            delayed_count: 0,
            non_delayed_count: 0,
        }));

        let mut sys = SingleThreadedGhostSystem::default();

        let test_clone = test.clone();
        sys.create_ref()
            .enqueue_processor(
                0,
                Box::new(move || {
                    test_clone.lock().delayed_count += 1;
                    Ok(GhostProcessInstructions::default()
                        .set_should_continue(true)
                        .set_next_run_delay_ms(40))
                }),
            )
            .unwrap();

        let test_clone = test.clone();
        sys.create_ref()
            .enqueue_processor(
                0,
                Box::new(move || {
                    test_clone.lock().non_delayed_count += 1;
                    Ok(GhostProcessInstructions::default().set_should_continue(true))
                }),
            )
            .unwrap();

        for _ in 0..10 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            sys.process().unwrap();
        }

        let test = test.lock();
        println!("delay_result {:?}", *test);
        assert!(
            test.non_delayed_count > test.delayed_count,
            "non-delayed should happend more often than delayed"
        );
    }
}