use crate::{error::Lib3hProtocolError, Address};
use std::convert::TryFrom;
use url::Url;

#[derive(Shrinkwrap, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[shrinkwrap(mutable)]
pub struct Lib3hUri(pub Url);

pub enum UriScheme {
    Agent,
    Transport,
    Memory,
    Undefined,
    Other(String),
}

static AGENT_SCHEME: &'static str = "agentid";
static TRANSPORT_SCHEME: &'static str = "transportid";
static MEMTRANSPORT_SCHEME: &'static str = "mem";
static UNDEFINED_SCHEME: &'static str = "none";

impl Lib3hUri {
    pub fn is_scheme(&self, scheme: UriScheme) -> bool {
        let s: String = scheme.into();
        self.scheme() == s
    }
    pub fn new_transport(machine_id: Address, agent_id: Address) -> Self {
        let url = Url::parse(&format!(
            "{}:{}?a={}",
            TRANSPORT_SCHEME, machine_id, agent_id
        ))
        .unwrap();
        Lib3hUri(url)
    }
}

impl From<UriScheme> for &str {
    fn from(scheme: UriScheme) -> &'static str {
        match scheme {
            UriScheme::Agent => AGENT_SCHEME,
            UriScheme::Transport => TRANSPORT_SCHEME,
            UriScheme::Memory => MEMTRANSPORT_SCHEME,
            UriScheme::Undefined => UNDEFINED_SCHEME,
            UriScheme::Other(_) => "",
        }
    }
}

impl From<UriScheme> for String {
    fn from(scheme: UriScheme) -> String {
        match scheme {
            UriScheme::Agent => AGENT_SCHEME.into(),
            UriScheme::Transport => TRANSPORT_SCHEME.into(),
            UriScheme::Memory => MEMTRANSPORT_SCHEME.into(),
            UriScheme::Undefined => UNDEFINED_SCHEME.into(),
            UriScheme::Other(s) => s.clone(),
        }
    }
}

impl TryFrom<&str> for Lib3hUri {
    type Error = Lib3hProtocolError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let url = Url::parse(s)?;
        Ok(Lib3hUri(url))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_uri_scheme_convert_str() {
        let s: &str = UriScheme::Agent.into();
        assert_eq!(s, AGENT_SCHEME);
        let s: &str = UriScheme::Transport.into();
        assert_eq!(s, TRANSPORT_SCHEME);
        let s: &str = UriScheme::Memory.into();
        assert_eq!(s, MEMTRANSPORT_SCHEME);
        let s: &str = UriScheme::Undefined.into();
        assert_eq!(s, UNDEFINED_SCHEME);
    }
    #[test]
    fn test_uri_scheme_convert_string() {
        let s: String = UriScheme::Agent.into();
        assert_eq!(s, AGENT_SCHEME.to_string());
        let s: String = UriScheme::Transport.into();
        assert_eq!(s, TRANSPORT_SCHEME.to_string());
        let s: String = UriScheme::Memory.into();
        assert_eq!(s, MEMTRANSPORT_SCHEME.to_string());
        let s: String = UriScheme::Undefined.into();
        assert_eq!(s, UNDEFINED_SCHEME.to_string());
        let s: String = UriScheme::Other("http".to_string()).into();
        assert_eq!(s, "http".to_string());
    }

    #[test]
    fn test_uri_from() {
        let uri = Lib3hUri::try_from("agentid:HcAsdkfjsdflkjsdf");
        assert_eq!(
            "Ok(Lib3hUri(\"agentid:HcAsdkfjsdflkjsdf\"))",
            format!("{:?}", uri)
        );

        let uri = Lib3hUri::try_from("badurl");
        assert_eq!(
            "Err(Lib3hProtocolError(UrlError(RelativeUrlWithoutBase)))",
            format!("{:?}", uri)
        );
    }

    #[test]
    fn test_uri_is_scheme() {
        let uri = Lib3hUri::try_from("agentid:HcAsdkfjsdflkjsdf").unwrap();
        assert!(uri.is_scheme(UriScheme::Agent));
        assert!(!uri.is_scheme(UriScheme::Transport));
        let uri = Lib3hUri::try_from("ws:x").unwrap();
        assert!(!uri.is_scheme(UriScheme::Agent));
        assert!(uri.is_scheme(UriScheme::Other("ws".to_string())));
        assert!(!uri.is_scheme(UriScheme::Other("http".to_string())));
    }

    #[test]
    fn test_uri_create_transport() {
        let machine_id = "fake_machine_id".into();
        let agent_id = "HcAfake_agent_id".into();
        let uri = Lib3hUri::new_transport(machine_id, agent_id);
        assert_eq!(
            "Lib3hUri(\"transportid:fake_machine_id?a=HcAfake_agent_id\")",
            format!("{:?}", uri)
        );
    }
}
