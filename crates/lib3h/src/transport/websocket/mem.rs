use crate::{
    mem_stream::*,
    transport::{
        error::*,
        websocket::{streams::*, tls::TlsConfig, wss_info::WssInfo},
    },
};
use url2::prelude::*;

impl StreamManager<MemStream> {
    pub fn with_mem_stream(tls_config: TlsConfig) -> Self {
        let bind: Bind<MemStream> = Box::new(move |url| Self::mem_bind(&Url2::from(url)));
        StreamManager::new(
            |uri| Ok(MemStream::connect(&Url2::parse(uri))?),
            bind,
            tls_config,
        )
    }

    fn mem_bind(url: &Url2) -> TransportResult<Acceptor<MemStream>> {
        if "mem" != url.scheme() {
            return Err("mem bind: url scheme must be mem".into());
        }
        if url.port().is_some() {
            return Err("mem bind: port must not be set".into());
        }
        if url.host_str().is_none() {
            return Err("mem bind: host_str must be set".into());
        }
        let mut url = url.clone();
        url.set_port(Some(4242)).map_err(|()| TransportError::from("failed to set port"))?;

        let mut listener = MemListener::bind(&url)?;
        Ok(Box::new(move || match listener.accept() {
            Ok(stream) => Ok(WssInfo::server(stream.get_url().clone().into(), stream)),
            Err(ref err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                Err(TransportError::new_kind(ErrorKind::Ignore(err.to_string())))
            }
            Err(e) => Err(e.into()),
        }))
    }
}
