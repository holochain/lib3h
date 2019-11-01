use crate::{
    mem_stream::*,
    transport::{
        error::*,
        websocket::{streams::*, tls::TlsConfig, wss_info::WssInfo},
    },
};
use url::Url;

impl StreamManager<MemStream> {
    pub fn with_mem_stream(tls_config: TlsConfig) -> Self {
        let bind: Bind<MemStream> = Box::new(move |url| Self::mem_bind(url));
        StreamManager::new(
            |uri| Ok(MemStream::connect(&Url::parse(uri).unwrap())?),
            bind,
            tls_config,
        )
    }

    fn mem_bind(url: &Url) -> TransportResult<Acceptor<MemStream>> {
        let mut listener = MemListener::bind(url)?;
        Ok(Box::new(move || match listener.accept() {
            Ok(stream) => Ok(WssInfo::server(stream.get_url().clone(), stream)),
            Err(ref err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                Err(TransportError::new_kind(ErrorKind::Ignore(err.to_string())))
            }
            Err(e) => Err(e.into()),
        }))
    }
}
