use crate::{error::Lib3hProtocolError, types::*};
use std::convert::{TryFrom, TryInto};
use url::Url;

//--------------------------------------------------------------------------------------------------
// UriScheme
//--------------------------------------------------------------------------------------------------

static AGENT_SCHEME: &'static str = "agentpubkey";
static NODE_SCHEME: &'static str = "nodepubkey";
static MEMORY_SCHEME: &'static str = "mem";
static UNDEFINED_SCHEME: &'static str = "none";

pub enum UriScheme {
    Undefined,
    Agent,
    Node,
    Memory,
    Other(String),
}

impl From<UriScheme> for &str {
    fn from(scheme: UriScheme) -> &'static str {
        match scheme {
            UriScheme::Undefined => UNDEFINED_SCHEME,
            UriScheme::Agent => AGENT_SCHEME,
            UriScheme::Node => NODE_SCHEME,
            UriScheme::Memory => MEMORY_SCHEME,
            UriScheme::Other(_) => "",
        }
    }
}

impl From<UriScheme> for String {
    fn from(scheme: UriScheme) -> String {
        match scheme {
            UriScheme::Undefined => UNDEFINED_SCHEME.into(),
            UriScheme::Agent => AGENT_SCHEME.into(),
            UriScheme::Node => NODE_SCHEME.into(),
            UriScheme::Memory => MEMORY_SCHEME.into(),
            UriScheme::Other(s) => s.clone(),
        }
    }
}

//--------------------------------------------------------------------------------------------------
// Lib3hUri
//--------------------------------------------------------------------------------------------------

#[derive(Shrinkwrap, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[shrinkwrap(mutable)]
pub struct Lib3hUri(pub Url);

impl Lib3hUri {
    // -- Constructors -- //

    pub fn with_node_and_agent_id(node_id: &NodePubKey, agent_id: &AgentPubKey) -> Self {
        let url = Self::parse(&format!("{}:{}?a={}", NODE_SCHEME, node_id, agent_id));
        Lib3hUri(url)
    }
    pub fn with_node_id(node_id: &NodePubKey) -> Self {
        let url = Self::parse(&format!("{}:{}", NODE_SCHEME, node_id));
        Lib3hUri(url)
    }
    pub fn with_agent_id(agent_id: &AgentPubKey) -> Self {
        let url = Self::parse(&format!("{}:{}", AGENT_SCHEME, agent_id));
        Lib3hUri(url)
    }
    pub fn with_undefined() -> Self {
        let url = Self::parse(&format!("{}:", UNDEFINED_SCHEME));
        Lib3hUri(url)
    }
    pub fn with_memory(other: &str) -> Self {
        let url = Self::parse(&format!("{}://{}", MEMORY_SCHEME, other));
        Lib3hUri(url)
    }

    // -- Misc -- //

    /// does this uri match the given scheme?
    pub fn is_scheme(&self, scheme: UriScheme) -> bool {
        let s: String = scheme.into();
        self.scheme() == s
    }

    /// True if its a TransportUri
    pub fn is_transport(&self) -> bool {
        let this_scheme = self.scheme();
        this_scheme != UNDEFINED_SCHEME && this_scheme != NODE_SCHEME && this_scheme != AGENT_SCHEME
    }

    /// new uri from &str
    fn parse(url_str: &str) -> Url {
        Url::parse(url_str).unwrap_or_else(|_| panic!("Invalid url format: '{}'", url_str))
    }

    /// The port portion of the url, if present.
    pub fn port(&self) -> Option<u16> {
        self.0.port()
    }

    /// The host portion of the url, if present.
    pub fn host(&self) -> Option<url::Host<&str>> {
        self.0.host()
    }

    /// The raw scheme name of the url as string. Eg. `mem` or `wss`.
    pub fn raw_scheme(&self) -> &str {
        self.0.scheme()
    }

    /// The hostname portion of the url (eg. `127.0.0.1` or `foo.com`), if present.
    pub fn hostname(&self) -> Option<String> {
        self.host().map(|host| host.to_string())
    }

    /// Produces a copy of this `Lib3hUri` with the given port set.
    /// Panics for out of range port values.
    pub fn with_port(&self, port: u16) -> Self {
        Builder::with_url(self.clone()).with_port(port).build()
    }

    /// set a higher-level agent_id i.e. ?a=agent_id
    pub fn set_agent_id(&mut self, agent_id: &AgentPubKey) {
        assert!(self.is_scheme(UriScheme::Node));
        self.0
            .query_pairs_mut()
            .clear()
            .append_pair("a", &agent_id.to_string());
    }

    /// clear any higher-level agent_id
    pub fn clear_agent_id(&mut self) {
        //assert!(self.is_scheme(UriScheme::Node), "{:?}", self);
        self.0.set_query(None);
    }

    /// do we have a higher-level agent_id? i.e. ?a=agent_id
    pub fn get_agent_id(&self) -> Option<AgentPubKey> {
        if !self.is_scheme(UriScheme::Node) {
            return None;
        }
        for (n, v) in self.0.query_pairs() {
            if &n == "a" {
                return Some(v.to_string().as_str().into());
            }
        }
        None
    }

    pub fn node_id(&self) -> NodePubKey {
        assert!(self.is_scheme(UriScheme::Node), "{:?}", self);
        self.0.path().into()
    }

    pub fn agent_id(&self) -> AgentPubKey {
        assert!(self.is_scheme(UriScheme::Agent), "{:?}", self);
        self.0.path().into()
    }
}

/// Eases building of a `Lib3hUri` with a fluent api. Users need not
/// ever mutate a `Lib3hUri` directly except for efficiency purposes. Instead,
/// let this builder be the only place where urls are manipulated.
#[derive(Debug, Clone)]
pub struct Builder {
    url: url::Url,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            url: Lib3hUri::with_undefined().into(),
        }
    }

    /// Primes a builder with the given url.
    pub fn with_url<T: Into<Lib3hUri>>(url: T) -> Self {
        let builder = Builder { url: url.into().0 };
        builder
    }

    /// Primes a builder with a raw url (such as a string).
    pub fn with_raw_url<T: TryInto<Lib3hUri>>(url: T) -> Result<Self, T::Error> {
        url.try_into().map(|url| Builder { url: url.0 })
    }

    pub fn with_host(&mut self, host: &str) -> &mut Self {
        self.url
            .set_host(Some(host))
            .unwrap_or_else(|e| panic!("Error setting host {:?}: {:?}", host, e));
        self
    }

    pub fn with_scheme(&mut self, scheme: &str) -> &mut Self {
        self.url
            .set_scheme(scheme)
            .unwrap_or_else(|e| panic!("Error setting scheme {:?}: {:?}", scheme, e));
        self
    }

    /// Sets the port. Will panic for out of range ports.
    pub fn with_port(&mut self, port: u16) -> &mut Self {
        self.url
            .set_port(Some(port))
            .unwrap_or_else(|e| panic!("Error setting port {:?}: {:?}", port, e));
        self
    }

    pub fn build(&self) -> Lib3hUri {
        self.url.clone().into()
    }
}

// -- Converters -- //

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
        let s: &str = UriScheme::Node.into();
        assert_eq!(s, NODE_SCHEME);
        let s: &str = UriScheme::Memory.into();
        assert_eq!(s, MEMORY_SCHEME);
        let s: &str = UriScheme::Undefined.into();
        assert_eq!(s, UNDEFINED_SCHEME);
    }
    #[test]
    fn test_uri_scheme_convert_string() {
        let s: String = UriScheme::Agent.into();
        assert_eq!(s, AGENT_SCHEME.to_string());
        let s: String = UriScheme::Node.into();
        assert_eq!(s, NODE_SCHEME.to_string());
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
        let id: AgentPubKey = "HcAsdkfjsdflkjsdf".into();
        let uri = Lib3hUri::with_agent_id(&id);
        let roundtrip: AgentPubKey = uri.agent_id();
        assert_eq!(roundtrip, id);
        let id: NodePubKey = "HcAsdkfjsdflkjsdf".into();
        let uri = Lib3hUri::with_node_id(&id);
        let roundtrip: NodePubKey = uri.node_id();
        assert_eq!(roundtrip, id);
    }

    #[test]
    fn test_uri_is_scheme() {
        let uri = Lib3hUri::try_from("agentpubkey:HcAsdkfjsdflkjsdf").unwrap();
        assert!(uri.is_scheme(UriScheme::Agent));
        assert!(!uri.is_scheme(UriScheme::Node));
        let uri = Lib3hUri::try_from("ws:x").unwrap();
        assert!(!uri.is_scheme(UriScheme::Agent));
        assert!(uri.is_scheme(UriScheme::Other("ws".to_string())));
        assert!(!uri.is_scheme(UriScheme::Other("http".to_string())));
    }

    #[test]
    fn test_uri_create_transport() {
        let node_id: NodePubKey = "fake_node_id".into();
        let agent_id: AgentPubKey = "HcAfake_agent_id".into();
        let mut uri = Lib3hUri::with_node_and_agent_id(&node_id, &agent_id);
        assert_eq!(
            "Lib3hUri(\"nodepubkey:fake_node_id?a=HcAfake_agent_id\")",
            format!("{:?}", uri)
        );
        assert_eq!(Some("HcAfake_agent_id".into()), uri.get_agent_id());
        uri.set_agent_id(&"bla".into());
        assert_eq!(Some("bla".into()), uri.get_agent_id());
        assert_eq!(NodePubKey::from("fake_node_id"), uri.node_id());
        uri.clear_agent_id();
        assert_eq!(None, uri.get_agent_id());
    }

    #[test]
    fn test_uri_builder() {
        let scheme = "wss";
        let host = "ws1://127.0.0.1/";
        let port = 9000;
        let url = Builder::with_raw_url(host)
            .unwrap_or_else(|e| panic!("with_raw_url: {:?}", e))
            //            .with_host(host)
            .with_scheme(scheme)
            .with_port(port)
            .build();

        assert_eq!(url.to_string(), "wss://127.0.0.1:9000/");
    }
}
