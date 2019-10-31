use crate::error::*;
use url::Url;
use lib3h_ghost_actor::GhostMutex;
use std::{
    collections::HashMap,
    io::{Read, Write},
};

struct MemManager {
    listeners: HashMap<Url, crossbeam_channel::Sender<MemStream>>,
}

impl MemManager {
    fn new() -> Self {
        Self {
            listeners: HashMap::new(),
        }
    }

    fn bind(&mut self, url: Url) -> Lib3hResult<MemListener> {
        Err("nope".into())
    }

    fn connect(&mut self, url: Url) -> Lib3hResult<MemStream> {
        Err("nope".into())
    }
}

lazy_static! {
    static ref MEM_MANAGER: GhostMutex<MemManager> = {
        GhostMutex::new(MemManager::new())
    };
}

pub struct MemListener {
    recv: crossbeam_channel::Receiver<MemStream>,
}

impl MemListener {
    pub fn bind(url: Url) -> Lib3hResult<MemListener> {
        MEM_MANAGER.lock().bind(url)
    }
}

fn create_mem_stream_pair() -> (MemStream, MemStream) {
    let (send1, recv1) = crossbeam_channel::unbounded();
    let (send2, recv2) = crossbeam_channel::unbounded();
    (
        MemStream::priv_new(send1, recv2),
        MemStream::priv_new(send2, recv1),
    )
}

pub struct MemStream {
    send: crossbeam_channel::Sender<Vec<u8>>,
    recv: crossbeam_channel::Receiver<Vec<u8>>,
    recv_buf: Vec<u8>,
}

impl MemStream {
    fn priv_new(
        send: crossbeam_channel::Sender<Vec<u8>>,
        recv: crossbeam_channel::Receiver<Vec<u8>>,
    ) -> MemStream {
        MemStream {
            send,
            recv,
            recv_buf: Vec::new(),
        }
    }

    pub fn connect(url: Url) -> Lib3hResult<MemStream> {
        MEM_MANAGER.lock().connect(url)
    }
}

impl Read for MemStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut disconnected = false;
        loop {
            match self.recv.try_recv() {
                Ok(mut data) => {
                    self.recv_buf.append(&mut data);
                }
                Err(crossbeam_channel::TryRecvError::Empty) => break,
                Err(crossbeam_channel::TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }
        if self.recv_buf.len() == 0 {
            if disconnected {
                return Ok(0);
            } else {
                return Err(std::io::ErrorKind::WouldBlock.into());
            }
        }

        let v: Vec<u8> = self
            .recv_buf
            .drain(0..std::cmp::min(buf.len(), self.recv_buf.len()))
            .collect();
        buf[0..v.len()].copy_from_slice(&v);
        Ok(v.len())
    }
}

impl Write for MemStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self.send.send(buf.to_vec()) {
            Ok(_) => Ok(buf.len()),
            Err(_) => Err(std::io::ErrorKind::NotConnected.into()),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_test_mem_stream() {
    }
}
