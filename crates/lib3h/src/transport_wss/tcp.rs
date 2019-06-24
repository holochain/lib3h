//! abstraction for working with Websocket connections
//! TcpStream specific functions

use crate::transport_wss::{
    Acceptor, Bind, TransportIdFactory, TransportInfo, TransportResult, TransportWss,
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

    fn tcp_bind(url: &str) -> TransportResult<Acceptor<TcpStream>> {
        TcpListener::bind(url)
            .map_err(|err| err.into())
            .and_then(move |listener: TcpListener| {
                listener
                    .set_nonblocking(true)
                    .map_err(|err| err.into())
                    .map(|()| {
                        let acceptor: Acceptor<TcpStream> =
                            Box::new(move |mut transport_id_factory: TransportIdFactory| {
                                let transport_id = transport_id_factory();
                                listener.accept().map_err(|err| err.into()).and_then(
                                    |(tcp_stream, socket_address)| {
                                        url::Url::parse(format!("{}", socket_address).as_str())
                                            .map(|url| {
                                                TransportInfo::new(transport_id, url, tcp_stream)
                                            })
                                            .map_err(|err| err.into())
                                    },
                                )
                            });
                        acceptor
                    })
            })
    }
}
