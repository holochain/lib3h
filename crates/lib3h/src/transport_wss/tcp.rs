//! abstraction for working with Websocket connections
//! TcpStream specific functions

use crate::{
    transport::error::TransportResult,
    transport_wss::{Acceptor, Bind, ConnectionIdFactory, IdGenerator, TransportWss, WssInfo},
};

use std::net::{TcpListener, TcpStream};

impl TransportWss<std::net::TcpStream> {
    /// convenience constructor for creating a websocket "Transport"
    /// instance that is based of the rust std TcpStream
    pub fn with_std_tcp_stream() -> Self {
        let bind: Bind<TcpStream> = Box::new(move |url| Self::tcp_bind(url));
        TransportWss::new(
            |uri| {
                let socket = std::net::TcpStream::connect(uri)?;
                socket.set_nonblocking(true)?;
                Ok(socket)
            },
            bind,
        )
    }

    fn tcp_bind(url: &url::Url) -> TransportResult<Acceptor<TcpStream>> {
        let host = url.host_str().expect("host name must be supplied");
        let port = url.port().unwrap_or(80);
        let formatted_url = format!("{}:{}", host, port);
        debug!("formatted url: {}", formatted_url);
        TcpListener::bind(formatted_url)
            .map_err(|err| err.into())
            .and_then(move |listener: TcpListener| {
                listener
                    .set_nonblocking(true)
                    .map_err(|err| {
                        error!("transport_wss::tcp listener error: {:?}", err);
                        err.into()
                    })
                    .map(|()| {
                        let acceptor: Acceptor<TcpStream> =
                            Box::new(move |mut connection_id_factory: ConnectionIdFactory| {
                                let connection_id = connection_id_factory.next_id();
                                listener
                                    .accept()
                                    .map_err(|err| {
                                        error!("transport_wss::tcp accept error: {:?}", err);
                                        err.into()
                                    })
                                    .and_then(|(tcp_stream, socket_address)| {
                                        let v4_socket_address = format!(
                                            "wss://{}:{}",
                                            socket_address.ip(),
                                            socket_address.port()
                                        );

                                        debug!(
                                            "transport_wss::tcp v4 socket_address: {}",
                                            v4_socket_address
                                        );
                                        url::Url::parse(v4_socket_address.as_str())
                                            .map(|url| {
                                                error!(
                                                    "transport_wss::tcp accepted for url {}",
                                                    url.clone()
                                                );
                                                WssInfo::server(
                                                    connection_id,
                                                    url.clone(),
                                                    tcp_stream,
                                                )
                                            })
                                            .map_err(|err| {
                                                error!("transport_wss::tcp url error: {:?}", err);
                                                err.into()
                                            })
                                    })
                            });
                        acceptor
                    })
            })
    }
}
