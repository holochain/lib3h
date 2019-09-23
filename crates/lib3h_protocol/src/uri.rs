use crate::{error::Lib3hProtocolError, Address};
use std::convert::TryFrom;
use url::Url;

static AGENT_SCHEME: &'static str = "agentid";
static TRANSPORT_SCHEME: &'static str = "transportid";
static MEMORY_SCHEME: &'static str = "mem";
static UNDEFINED_SCHEME: &'static str = "none";

///////////////////////////////////
/// UriScheme
///////////////////////////////////
///
pub enum UriScheme {
    Agent,
    Transport,
    Memory,
    Undefined,
    Other(String),
}

impl From<UriScheme> for &str {
    fn from(scheme: UriScheme) -> &'static str {
        match scheme {
            UriScheme::Agent => AGENT_SCHEME,
            UriScheme::Transport => TRANSPORT_SCHEME,
            UriScheme::Memory => MEMORY_SCHEME,
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
            UriScheme::Memory => MEMORY_SCHEME.into(),
            UriScheme::Undefined => UNDEFINED_SCHEME.into(),
            UriScheme::Other(s) => s.clone(),
        }
    }
}

///////////////////////////////////
/// Lib3hUri
///////////////////////////////////

#[derive(Shrinkwrap, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[shrinkwrap(mutable)]
pub struct Lib3hUri(pub Url);

impl Lib3hUri {
    pub fn is_scheme(&self, scheme: UriScheme) -> bool {
        let s: String = scheme.into();
        self.scheme() == s
    }
    pub fn with_transport_id(transport_id: &Address, agent_id: &Address) -> Self {
        let url = Url::parse(&format!(
            "{}:{}?a={}",
            TRANSPORT_SCHEME, transport_id, agent_id
        ))
        .unwrap();
        Lib3hUri(url)
    }
    pub fn with_agent_id(agent_id: &Address) -> Self {
        let url = Url::parse(&format!("{}:{}", AGENT_SCHEME, agent_id)).unwrap();
        Lib3hUri(url)
    }
    pub fn with_undefined(other: &str) -> Self {
        let url = Url::parse(&format!("{}:{}", UNDEFINED_SCHEME, other)).unwrap();
        Lib3hUri(url)
    }
    pub fn with_memory(other: &str) -> Self {
        let url = Url::parse(&format!("{}://{}", MEMORY_SCHEME, other)).unwrap();
        Lib3hUri(url)
    }
}

impl TryFrom<&str> for Lib3hUri {
    type Error = Lib3hProtocolError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let url = Url::parse(s)?;
        Ok(Lib3hUri(url))
    }
}

impl From<Lib3hUri> for Url {
    fn from(u: Lib3hUri) -> Url {
        u.0
    }
}

impl From<Url> for Lib3hUri {
    fn from(u: Url) -> Lib3hUri {
        Lib3hUri(u)
    }
}

impl From<Lib3hUri> for Address {
    fn from(u: Lib3hUri) -> Address {
        if !u.is_scheme(UriScheme::Agent) {
            panic!("Can't convert a non agentId Lib3hUri into an address")
        }
        u.path().into()
    }
}

impl std::fmt::Display for Lib3hUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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
        assert_eq!(s, MEMORY_SCHEME);
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
        assert_eq!(s, MEMORY_SCHEME.to_string());
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
    fn test_address_from_uri() {
        let agent_id: Address = "HcAsdkfjsdflkjsdf".into();
        let uri = Lib3hUri::with_agent_id(&agent_id);
        let roundtrip: Address = uri.into();
        assert_eq!(roundtrip, agent_id);
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
        let transport_id: Address = "fake_transport_id".into();
        let agent_id: Address = "HcAfake_agent_id".into();
        let uri = Lib3hUri::with_transport_id(&transport_id, &agent_id);
        assert_eq!(
            "Lib3hUri(\"transportid:fake_transport_id?a=HcAfake_agent_id\")",
            format!("{:?}", uri)
        );
    }
}
