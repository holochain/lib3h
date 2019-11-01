use lib3h_ghost_actor::GhostMutex;
use std::{
    collections::{hash_map::Entry, HashMap},
    io::{Read, Write},
};
use url::Url;

// -- mem listener -- //

#[derive(Debug)]
/// equivalent to TcpListener, the network interface bind point
pub struct MemListener {
    url: Url,
    recv: crossbeam_channel::Receiver<MemStream>,
    accept_queue: Vec<MemStream>,
}

impl MemListener {
    /// private constructor, you probably want `bind`
    fn priv_new(url: Url, recv: crossbeam_channel::Receiver<MemStream>) -> Self {
        Self {
            url,
            recv,
            accept_queue: Vec::new(),
        }
    }

    /// bind to a virtual "memory" interface
    pub fn bind(url: &Url) -> std::io::Result<MemListener> {
        MEM_MANAGER.lock().bind(url)
    }

    /// get the url bound to
    pub fn get_url(&self) -> &Url {
        &self.url
    }

    /// accept a stream on this listener interface
    /// this is non-blocking, and will return WouldBlock if no connections
    /// will return BrokenPipe if our channel somehow got disconnected
    pub fn accept(&mut self) -> std::io::Result<MemStream> {
        loop {
            // first, drain all pending connections from our recv channel
            match self.recv.try_recv() {
                Ok(stream) => {
                    self.accept_queue.push(stream);
                }
                Err(crossbeam_channel::TryRecvError::Empty) => break,
                Err(crossbeam_channel::TryRecvError::Disconnected) => {
                    // wait until our user has accepted all pending connections
                    // before letting them know the channel is broken
                    if self.accept_queue.is_empty() {
                        return Err(std::io::ErrorKind::BrokenPipe.into());
                    }
                }
            }
        }
        if self.accept_queue.is_empty() {
            // acceptor is non-blocking we have nothing to return
            return Err(std::io::ErrorKind::WouldBlock.into());
        }
        // pull the next item off the queue
        Ok(self.accept_queue.remove(0))
    }
}

impl Drop for MemListener {
    fn drop(&mut self) {
        MEM_MANAGER.lock().unbind(&self.url);
    }
}

// -- mem stream -- //

#[derive(Debug)]
/// equivalent to TcpStream, represents one end of a virtual memory connection
pub struct MemStream {
    url: Url,
    send: crossbeam_channel::Sender<Vec<u8>>,
    recv: crossbeam_channel::Receiver<Vec<u8>>,
    recv_buf: Vec<u8>,
}

impl MemStream {
    /// private constructor, you probably want `connect`
    fn priv_new(
        url: Url,
        send: crossbeam_channel::Sender<Vec<u8>>,
        recv: crossbeam_channel::Receiver<Vec<u8>>,
    ) -> MemStream {
        MemStream {
            url,
            send,
            recv,
            recv_buf: Vec::new(),
        }
    }

    /// connect to a virtual memory listening interface
    /// will return ConnectionRefused if there is not one
    pub fn connect(url: &Url) -> std::io::Result<MemStream> {
        MEM_MANAGER.lock().connect(url)
    }

    /// get the Url we are connected to
    pub fn get_url(&self) -> &Url {
        &self.url
    }
}

impl Read for MemStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut disconnected = false;
        loop {
            // first, drain everything from our channel
            match self.recv.try_recv() {
                Ok(mut data) => {
                    self.recv_buf.append(&mut data);
                }
                Err(crossbeam_channel::TryRecvError::Empty) => break,
                Err(crossbeam_channel::TryRecvError::Disconnected) => {
                    // if our channel is broken, we will consider it EOF
                    disconnected = true;
                    break;
                }
            }
        }
        if self.recv_buf.len() == 0 {
            if disconnected {
                // nothing in our buffer, let the user know about the EOF
                return Ok(0);
            } else {
                // nothing in our buffer, but our channel is still active
                // let them know that we have no data without blocking
                return Err(std::io::ErrorKind::WouldBlock.into());
            }
        }

        // drain as much as we have and / or the user can take
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
        // if we're still connected, send data to our pair
        match self.send.send(buf.to_vec()) {
            Ok(_) => Ok(buf.len()),
            Err(_) => Err(std::io::ErrorKind::NotConnected.into()),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// -- utility functions -- //

/// private stream pair constructor, these streams can message each other
fn create_mem_stream_pair(url: Url) -> (MemStream, MemStream) {
    let (send1, recv1) = crossbeam_channel::unbounded();
    let (send2, recv2) = crossbeam_channel::unbounded();
    (
        MemStream::priv_new(url.clone(), send1, recv2),
        MemStream::priv_new(url, send2, recv1),
    )
}

// -- singleton memory manager -- //

/// private singleton for managing virtual memory listening interfaces
struct MemManager {
    listeners: HashMap<Url, crossbeam_channel::Sender<MemStream>>,
}

impl MemManager {
    /// create a new singleton
    fn new() -> Self {
        Self {
            listeners: HashMap::new(),
        }
    }

    /// manage binding a new MemListener interface
    fn bind(&mut self, url: &Url) -> std::io::Result<MemListener> {
        match self.listeners.entry(url.clone()) {
            Entry::Occupied(_) => Err(std::io::ErrorKind::AddrInUse.into()),
            Entry::Vacant(e) => {
                // the url is not in use, let's create a new listener
                let (send, recv) = crossbeam_channel::unbounded();
                e.insert(send);
                Ok(MemListener::priv_new(url.clone(), recv))
            }
        }
    }

    /// unbind a previously bound MemListener interface (happens on Drop)
    fn unbind(&mut self, url: &Url) {
        self.listeners.remove(url);
    }

    /// connect to an existing MemListener interface
    fn connect(&mut self, url: &Url) -> std::io::Result<MemStream> {
        let mut disconnected = false;
        if let Entry::Occupied(mut e) = self.listeners.entry(url.clone()) {
            // there is a listener bound to this url
            // create a new stream pair
            // send one to the listener's accept queue
            // return the other one
            let (one, two) = create_mem_stream_pair(url.clone());
            // if the send fails, we must have a broken listener connection
            // we'll clean that up after
            match e.get_mut().send(one) {
                Ok(_) => return Ok(two),
                Err(_) => disconnected = true,
            }
        }
        if disconnected {
            self.listeners.remove(url);
        }
        Err(std::io::ErrorKind::ConnectionRefused.into())
    }
}

// this is the actual singleton global reference
lazy_static! {
    static ref MEM_MANAGER: GhostMutex<MemManager> = { GhostMutex::new(MemManager::new()) };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// create a unique listener && establish connection pair
    fn setup() -> (MemListener, MemStream, MemStream) {
        let url = Url::parse(&format!("test:{}", nanoid::simple())).unwrap();
        let mut listener = MemListener::bind(&url).unwrap();
        let client = MemStream::connect(&url).unwrap();
        let server = listener.accept().unwrap();
        (listener, client, server)
    }

    #[test]
    fn it_should_connection_refused() {
        match MemStream::connect(&Url::parse("badconnection:").unwrap()) {
            Err(ref e) if e.kind() == std::io::ErrorKind::ConnectionRefused => (),
            _ => panic!("unexpected"),
        }
    }

    #[test]
    fn it_should_addr_in_use() {
        let (listener, _c, _s) = setup();
        match MemListener::bind(listener.get_url()) {
            Err(ref e) if e.kind() == std::io::ErrorKind::AddrInUse => (),
            _ => panic!("unexpected"),
        }
    }

    #[test]
    fn it_can_read_write() {
        let mut buf = [0_u8; 1024];
        let (_listener, mut client, mut server) = setup();

        client.write_all(b"test1").unwrap();

        assert_eq!(5, server.read(&mut buf).unwrap());
        assert_eq!(b"test1", &buf[..5]);

        server.write_all(b"test2").unwrap();

        assert_eq!(5, client.read(&mut buf).unwrap());
        assert_eq!(b"test2", &buf[..5]);
    }

    #[test]
    fn it_should_would_block() {
        let mut buf = [0_u8; 1024];
        let (mut listener, mut client, mut server) = setup();

        match listener.accept() {
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => (),
            _ => panic!("unexpected"),
        }

        match client.read(&mut buf) {
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => (),
            _ => panic!("unexpected"),
        }

        match server.read(&mut buf) {
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => (),
            _ => panic!("unexpected"),
        }
    }

    #[test]
    fn it_can_keep_talking_after_listener_drop() {
        let mut buf = [0_u8; 1024];
        let (mut client, mut server) = {
            let (_listener, client, server) = setup();
            (client, server)
        };

        client.write_all(b"test1").unwrap();

        assert_eq!(5, server.read(&mut buf).unwrap());
        assert_eq!(b"test1", &buf[..5]);

        server.write_all(b"test2").unwrap();

        assert_eq!(5, client.read(&mut buf).unwrap());
        assert_eq!(b"test2", &buf[..5]);
    }

    #[test]
    fn it_should_end_of_stream() {
        let mut buf = [0_u8; 1024];
        let mut server = {
            let (_listener, mut client, server) = setup();
            client.write_all(b"test1").unwrap();
            server
        };

        assert_eq!(5, server.read(&mut buf).unwrap());
        assert_eq!(b"test1", &buf[..5]);

        match server.read(&mut buf) {
            Ok(0) => (),
            _ => panic!("unexpected"),
        }
    }
}
