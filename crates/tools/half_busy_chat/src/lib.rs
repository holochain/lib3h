//! HalfBusyChat is a simulation test framework backend for lib3h
//! - See the integration tests for automated usage
//! - See half_busy_chat_cli for manual simulation

extern crate crossbeam_channel;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageData {
    pub from_address: String,
    pub payload: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatEvent {
    Message(MessageData),
}

pub type HandleEvent = Box<dyn FnMut(ChatEvent) + Send>;

pub struct HalfBusyChat {
    thread: Option<std::thread::JoinHandle<()>>,
    thread_continue: Arc<AtomicBool>,
    out_send: crossbeam_channel::Sender<ChatEvent>,
}

impl Drop for HalfBusyChat {
    fn drop(&mut self) {
        self.thread_continue.store(false, Ordering::Relaxed);
        std::mem::replace(&mut self.thread, None)
            .expect("cannot drop, thread already None")
            .join()
            .expect("thead join failed");
    }
}

impl HalfBusyChat {
    pub fn new(mut handler: HandleEvent) -> Self {
        let thread_continue = Arc::new(AtomicBool::new(true));

        let (out_send, out_recv) = crossbeam_channel::unbounded();

        let thread_continue_inner = thread_continue.clone();
        Self {
            thread: Some(std::thread::spawn(move || {
                while thread_continue_inner.load(Ordering::Relaxed) {
                    if let Ok(msg) = out_recv.try_recv() {
                        // just echo it as a stub
                        handler(msg);
                    }

                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            })),
            thread_continue,
            out_send,
        }
    }

    pub fn send(&mut self, event: ChatEvent) {
        self.out_send.send(event).expect("send fail");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_echo() {
        let (s, r) = crossbeam_channel::unbounded();
        let mut chat = HalfBusyChat::new(Box::new(move |event| {
            s.send(event).expect("send fail");
        }));

        let msg = ChatEvent::Message(MessageData {
            from_address: "addr".to_string(),
            payload: "yo".to_string(),
        });

        chat.send(msg.clone());

        assert_eq!(msg, r.recv().expect("receive fail"));
    }
}
