use crate::transport::websocket::{streams::WebsocketStreamState, BaseStream, TransportResult};

/// Represents an individual connection
#[derive(Debug)]
pub struct WssInfo<T: std::io::Read + std::io::Write + std::fmt::Debug> {
    pub(in crate::transport::websocket) request_id: String,
    pub(in crate::transport::websocket) url: url::Url,
    pub(in crate::transport::websocket) last_msg: std::time::Instant,
    pub(in crate::transport::websocket) send_queue: Vec<Vec<u8>>,
    pub(in crate::transport::websocket) stateful_socket: WebsocketStreamState<T>,
}

impl<T: std::io::Read + std::io::Write + std::fmt::Debug> WssInfo<T> {
    pub fn close(&mut self) -> TransportResult<()> {
        if let WebsocketStreamState::ReadyWss(socket) = &mut self.stateful_socket {
            socket.close(None)?;
            socket.write_pending()?;
        }
        self.stateful_socket = WebsocketStreamState::None;
        Ok(())
    }

    pub fn new(url: url::Url, socket: BaseStream<T>, is_server: bool) -> Self {
        WssInfo {
            // TODO set a request id
            request_id: "".to_string(),
            url,
            last_msg: std::time::Instant::now(),
            send_queue: Vec::new(),
            stateful_socket: match is_server {
                false => WebsocketStreamState::Connecting(socket),
                true => WebsocketStreamState::ConnectingSrv(socket),
            },
        }
    }

    pub fn client(url: url::Url, socket: BaseStream<T>) -> Self {
        Self::new(url, socket, false)
    }

    pub fn server(url: url::Url, socket: BaseStream<T>) -> Self {
        Self::new(url, socket, true)
    }
}
