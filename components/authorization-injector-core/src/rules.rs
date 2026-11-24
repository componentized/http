//! Matching a request's origin to the configured authorization, without any WASI imports so it is
//! unit tested on the host.
//!
//! A rule's key is an origin, `scheme://host[:port]`. The host may start with `*.`, matching the
//! domain and every subdomain, e.g. `*.example.com` matches `example.com` and `a.b.example.com`.
//! The port may be `*`, matching any port, without a port the key matches the scheme's default
//! port.

/// The default port of a scheme, `None` for a scheme without one.
fn default_port(scheme: &str) -> Option<u16> {
    match scheme {
        "http" => Some(80),
        "https" => Some(443),
        _ => None,
    }
}

/// The origin of a request, normalized for matching.
#[derive(Debug, PartialEq, Eq)]
pub struct Origin {
    scheme: String,
    host: String,
    port: Option<u16>,
}

impl Origin {
    /// The origin of a request with the scheme and authority, `None` when the authority is not
    /// valid.
    ///
    /// Userinfo is ignored, the host is lowercase without a trailing dot, and a missing port is
    /// the scheme's default port.
    pub fn new(scheme: &str, authority: &str) -> Option<Self> {
        let scheme = scheme.to_ascii_lowercase();
        let host_port = match authority.rsplit_once('@') {
            Some((_userinfo, host_port)) => host_port,
            None => authority,
        };
        let (host, port) = split_host_port(host_port)?;
        let host = normalize_host(host)?;
        let port = match port {
            // `host:` is the default port, RFC 3986 section 3.2.3
            None | Some("") => default_port(&scheme),
            Some(port) => Some(parse_port(port)?),
        };
        Some(Self { scheme, host, port })
    }
}

/// The host and port of an authority, `[::1]:8080` is `[::1]` and `8080`.
fn split_host_port(host_port: &str) -> Option<(&str, Option<&str>)> {
    if host_port.starts_with('[') {
        let end = host_port.find(']')? + 1;
        let (host, rest) = host_port.split_at(end);
        return match rest {
            "" => Some((host, None)),
            rest => Some((host, Some(rest.strip_prefix(':')?))),
        };
    }
    Some(match host_port.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (host_port, None),
    })
}

/// A lowercase host without a trailing dot, `None` when empty.
fn normalize_host(host: &str) -> Option<String> {
    let host = host.strip_suffix('.').unwrap_or(host);
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

/// A port number, digits only.
fn parse_port(port: &str) -> Option<u16> {
    if port.is_empty() || !port.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    port.parse().ok()
}

#[derive(Debug, PartialEq, Eq)]
enum HostPattern {
    /// The host, e.g. `example.com`.
    Exact(String),
    /// The domain and its subdomains, e.g. `example.com` for `*.example.com`.
    Domain(String),
}

impl HostPattern {
    fn matches(&self, host: &str) -> bool {
        match self {
            Self::Exact(exact) => host == exact,
            Self::Domain(domain) => {
                host == domain
                    || host
                        .strip_suffix(domain.as_str())
                        .is_some_and(|sub| sub.ends_with('.'))
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum PortPattern {
    /// The port, `None` for a scheme without a default port.
    Exact(Option<u16>),
    /// Any port, `*`.
    Any,
}

/// How specific a matching rule is, the most specific rule is used. An exact host is more
/// specific than a domain, a longer domain more than a shorter one, then an exact port more than
/// any port.
type Specificity = (bool, usize, bool);

/// The authorization for requests to the origins matching the key.
#[derive(Debug)]
struct Rule {
    key: String,
    scheme: String,
    host: HostPattern,
    port: PortPattern,
    authorization: String,
}

impl Rule {
    fn parse(key: String, authorization: String) -> Result<Self, String> {
        let invalid_key = |error: &str| format!("KEY={key} ERROR={error}");

        let (scheme, host_port) = key
            .split_once("://")
            .ok_or_else(|| invalid_key("expected 'scheme://host[:port]'"))?;
        let valid_scheme = scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
        if !valid_scheme {
            return Err(invalid_key("invalid scheme"));
        }
        let scheme = scheme.to_ascii_lowercase();
        if host_port.contains(['/', '?', '#']) {
            return Err(invalid_key("expected an origin without a path, query or fragment"));
        }
        if host_port.contains('@') {
            return Err(invalid_key("expected an origin without userinfo"));
        }

        let (host, port) = split_host_port(host_port).ok_or_else(|| invalid_key("invalid host"))?;
        let host = match host.strip_prefix("*.") {
            Some(domain) => HostPattern::Domain(domain.to_string()),
            None => HostPattern::Exact(host.to_string()),
        };
        let host = match host {
            HostPattern::Exact(host) | HostPattern::Domain(host) if host.contains('*') => {
                return Err(invalid_key(
                    "a wildcard host must start with '*.', e.g. '*.example.com'",
                ));
            }
            HostPattern::Exact(host) => HostPattern::Exact(
                normalize_host(&host).ok_or_else(|| invalid_key("missing host"))?,
            ),
            HostPattern::Domain(domain) => HostPattern::Domain(
                normalize_host(&domain).ok_or_else(|| invalid_key("missing domain after '*.'"))?,
            ),
        };
        let port = match port {
            None | Some("") => PortPattern::Exact(default_port(&scheme)),
            Some("*") => PortPattern::Any,
            Some(port) => PortPattern::Exact(Some(
                parse_port(port).ok_or_else(|| invalid_key("invalid port, expected a number or '*'"))?,
            )),
        };

        // the value is a secret, the error never includes it
        let valid_value = authorization
            .bytes()
            .all(|b| b == b'\t' || (0x20..0x7f).contains(&b) || b >= 0x80);
        if !valid_value {
            return Err(invalid_key("invalid header value"));
        }

        Ok(Self {
            key,
            scheme,
            host,
            port,
            authorization,
        })
    }

    fn matches(&self, origin: &Origin) -> bool {
        self.scheme == origin.scheme
            && self.host.matches(&origin.host)
            && match self.port {
                PortPattern::Any => true,
                PortPattern::Exact(port) => port == origin.port,
            }
    }

    fn specificity(&self) -> Specificity {
        let (exact_host, host_len) = match &self.host {
            HostPattern::Exact(host) => (true, host.len()),
            HostPattern::Domain(domain) => (false, domain.len()),
        };
        (exact_host, host_len, self.port != PortPattern::Any)
    }

    /// Whether the rules match exactly the same origins, e.g. `https://example.com` and
    /// `https://example.com:443`.
    fn same_origins(&self, other: &Rule) -> bool {
        self.scheme == other.scheme && self.host == other.host && self.port == other.port
    }
}

/// The authorization configured for each origin.
#[derive(Debug)]
pub struct Rules(Vec<Rule>);

impl Rules {
    /// Parse the `wasi:config/store` entries, an error describes the invalid entry, e.g.
    /// `KEY=<key> ERROR=<error>`, without the value.
    pub fn parse(entries: Vec<(String, String)>) -> Result<Self, String> {
        let mut rules: Vec<Rule> = Vec::with_capacity(entries.len());
        for (key, value) in entries {
            let rule = Rule::parse(key, value)?;
            if let Some(other) = rules.iter().find(|other| other.same_origins(&rule)) {
                return Err(format!(
                    "KEY={} ERROR=matches the same origins as '{}'",
                    rule.key, other.key
                ));
            }
            rules.push(rule);
        }
        Ok(Self(rules))
    }

    /// The authorization for the origin, from the most specific matching rule. An empty value
    /// is no authorization, e.g. to exclude a host from a domain.
    pub fn authorization(&self, origin: &Origin) -> Option<&str> {
        self.0
            .iter()
            .filter(|rule| rule.matches(origin))
            .max_by_key(|rule| rule.specificity())
            .map(|rule| rule.authorization.as_str())
            .filter(|authorization| !authorization.is_empty())
    }
}

/// The key a method without a key of its own falls back to.
const ANY_METHOD: &str = "*";

/// Whether the authorization is added to requests with each method, e.g. `get=deferred` and
/// `*=denied`.
///
/// The key is the lowercase method, the value is `deferred`, or empty, or `denied`. A method
/// without a key uses the `*` key, and is denied without it, so a credential is only sent with
/// the methods it is meant for.
#[derive(Debug)]
pub struct MethodRules {
    allowed: Vec<(String, bool)>,
}

impl MethodRules {
    /// Parse the `wasi:config/store` entries, an error describes the invalid entry, e.g.
    /// `KEY=<key> VALUE=<value> ERROR=<error>`.
    pub fn parse(entries: Vec<(String, String)>) -> Result<Self, String> {
        let mut allowed = Vec::with_capacity(entries.len());
        for (key, value) in entries {
            let is_allowed = match value.as_str() {
                "" | "deferred" => true,
                "denied" => false,
                _ => {
                    return Err(format!(
                        "KEY={key} VALUE={value} ERROR=expected one of: 'deferred', 'denied'"
                    ));
                }
            };
            allowed.push((key.to_ascii_lowercase(), is_allowed));
        }
        Ok(Self { allowed })
    }

    /// Whether the authorization is added to a request with the method, any case.
    pub fn allows(&self, method: &str) -> bool {
        let method = method.to_ascii_lowercase();
        let lookup = |key: &str| {
            // the last value of a key wins, like `wasi:config/store#get`
            self.allowed
                .iter()
                .rev()
                .find(|(k, _)| k == key)
                .map(|(_, allowed)| *allowed)
        };
        lookup(&method).or_else(|| lookup(ANY_METHOD)).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn methods(entries: &[(&str, &str)]) -> MethodRules {
        MethodRules::parse(
            entries
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        )
        .unwrap()
    }

    #[test]
    fn methods_fall_back_to_any_then_denied() {
        let rules = methods(&[("get", "deferred"), ("post", ""), ("trace", "denied")]);
        assert!(rules.allows("get"));
        assert!(rules.allows("GET"));
        assert!(rules.allows("post"));
        assert!(!rules.allows("trace"));
        assert!(!rules.allows("bogus"));

        let rules = methods(&[("trace", "denied"), ("*", "deferred")]);
        assert!(rules.allows("bogus"));
        assert!(!rules.allows("trace"));

        assert!(!methods(&[]).allows("get"));
    }

    #[test]
    fn invalid_method_value() {
        assert_eq!(
            MethodRules::parse(vec![("post".into(), "allowed".into())]).unwrap_err(),
            "KEY=post VALUE=allowed ERROR=expected one of: 'deferred', 'denied'"
        );
    }

    fn rules(entries: &[(&str, &str)]) -> Rules {
        Rules::parse(
            entries
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        )
        .unwrap()
    }

    fn parse_error(key: &str, value: &str) -> String {
        Rules::parse(vec![(key.to_string(), value.to_string())]).unwrap_err()
    }

    fn lookup<'a>(rules: &'a Rules, scheme: &str, authority: &str) -> Option<&'a str> {
        rules.authorization(&Origin::new(scheme, authority).unwrap())
    }

    #[test]
    fn origin_defaults_port_and_normalizes_host() {
        assert_eq!(
            Origin::new("HTTPS", "user:pass@Example.COM."),
            Some(Origin {
                scheme: "https".into(),
                host: "example.com".into(),
                port: Some(443)
            })
        );
        assert_eq!(Origin::new("http", "example.com:").unwrap().port, Some(80));
        assert_eq!(Origin::new("ws", "example.com").unwrap().port, None);
        assert_eq!(
            Origin::new("http", "[::1]:8080"),
            Some(Origin {
                scheme: "http".into(),
                host: "[::1]".into(),
                port: Some(8080)
            })
        );
        assert_eq!(Origin::new("http", "example.com:http"), None);
        assert_eq!(Origin::new("http", ":80"), None);
        assert_eq!(Origin::new("http", "[::1"), None);
    }

    #[test]
    fn exact_origin() {
        let rules = rules(&[("https://example.com", "Bearer a")]);
        assert_eq!(lookup(&rules, "https", "example.com"), Some("Bearer a"));
        assert_eq!(lookup(&rules, "https", "EXAMPLE.com:443"), Some("Bearer a"));
        assert_eq!(lookup(&rules, "https", "example.com:8443"), None);
        assert_eq!(lookup(&rules, "http", "example.com"), None);
        assert_eq!(lookup(&rules, "https", "api.example.com"), None);
        assert_eq!(lookup(&rules, "https", "notexample.com"), None);
    }

    #[test]
    fn explicit_port() {
        let rules = rules(&[("http://localhost:8080", "Bearer a")]);
        assert_eq!(lookup(&rules, "http", "localhost:8080"), Some("Bearer a"));
        assert_eq!(lookup(&rules, "http", "localhost"), None);
    }

    #[test]
    fn wildcard_domain_matches_domain_and_subdomains() {
        let rules = rules(&[("https://*.example.com", "Bearer a")]);
        assert_eq!(lookup(&rules, "https", "example.com"), Some("Bearer a"));
        assert_eq!(lookup(&rules, "https", "foo.example.com"), Some("Bearer a"));
        assert_eq!(lookup(&rules, "https", "a.b.example.com"), Some("Bearer a"));
        assert_eq!(lookup(&rules, "https", "notexample.com"), None);
        assert_eq!(lookup(&rules, "https", "example.com.evil"), None);
        assert_eq!(lookup(&rules, "https", "foo.example.com:8443"), None);
    }

    #[test]
    fn wildcard_port() {
        let rules = rules(&[("http://localhost:*", "Bearer a")]);
        assert_eq!(lookup(&rules, "http", "localhost"), Some("Bearer a"));
        assert_eq!(lookup(&rules, "http", "localhost:8080"), Some("Bearer a"));
        assert_eq!(lookup(&rules, "https", "localhost"), None);
    }

    #[test]
    fn most_specific_rule_wins() {
        let rules = rules(&[
            ("https://*.example.com:*", "any port"),
            ("https://*.example.com", "domain"),
            ("https://*.api.example.com", "subdomain"),
            ("https://api.example.com:*", "exact host any port"),
            ("https://api.example.com", "exact"),
        ]);
        assert_eq!(lookup(&rules, "https", "example.com:8443"), Some("any port"));
        assert_eq!(lookup(&rules, "https", "www.example.com"), Some("domain"));
        assert_eq!(lookup(&rules, "https", "v1.api.example.com"), Some("subdomain"));
        assert_eq!(
            lookup(&rules, "https", "api.example.com:8443"),
            Some("exact host any port")
        );
        assert_eq!(lookup(&rules, "https", "api.example.com"), Some("exact"));
    }

    #[test]
    fn empty_value_excludes_an_origin() {
        let rules = rules(&[
            ("https://*.example.com", "Bearer a"),
            ("https://public.example.com", ""),
        ]);
        assert_eq!(lookup(&rules, "https", "public.example.com"), None);
        assert_eq!(lookup(&rules, "https", "api.example.com"), Some("Bearer a"));
    }

    #[test]
    fn invalid_keys() {
        assert_eq!(
            parse_error("example.com", "x"),
            "KEY=example.com ERROR=expected 'scheme://host[:port]'"
        );
        assert_eq!(
            parse_error("1http://example.com", "x"),
            "KEY=1http://example.com ERROR=invalid scheme"
        );
        assert_eq!(
            parse_error("https://example.com/path", "x"),
            "KEY=https://example.com/path ERROR=expected an origin without a path, query or fragment"
        );
        assert_eq!(
            parse_error("https://user@example.com", "x"),
            "KEY=https://user@example.com ERROR=expected an origin without userinfo"
        );
        assert_eq!(
            parse_error("https://foo*.example.com", "x"),
            "KEY=https://foo*.example.com ERROR=a wildcard host must start with '*.', e.g. '*.example.com'"
        );
        assert_eq!(
            parse_error("https://*", "x"),
            "KEY=https://* ERROR=a wildcard host must start with '*.', e.g. '*.example.com'"
        );
        assert_eq!(
            parse_error("https://*.", "x"),
            "KEY=https://*. ERROR=missing domain after '*.'"
        );
        assert_eq!(parse_error("https://", "x"), "KEY=https:// ERROR=missing host");
        assert_eq!(
            parse_error("https://example.com:99999", "x"),
            "KEY=https://example.com:99999 ERROR=invalid port, expected a number or '*'"
        );
    }

    #[test]
    fn invalid_value_is_not_logged() {
        assert_eq!(
            parse_error("https://example.com", "Bearer a\r\nx-injected: b"),
            "KEY=https://example.com ERROR=invalid header value"
        );
    }

    #[test]
    fn duplicate_origins() {
        let err = Rules::parse(vec![
            ("https://example.com".into(), "a".into()),
            ("https://EXAMPLE.com:443".into(), "b".into()),
        ])
        .unwrap_err();
        assert_eq!(
            err,
            "KEY=https://EXAMPLE.com:443 ERROR=matches the same origins as 'https://example.com'"
        );
    }
}
