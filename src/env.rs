// Copyright (c) 2022 Sebastian Wiesner <sebastian@swsnr.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Resolve proxies via environment variables.
//!
//! This module provides a proxy resolver using the standard HTTP proxy environment variables, as
//! well as the specific parts of that proxy resolver.
//!
//! The [`EnvProxyResolver::from_curl_env`] provides a proxy resolver which uses the [curl](https://curl.se/)
//! environment variables to resolve a proxy for a given URL.
//!
//! It consists of a [`EnvProxies`] struct which extracts the actual proxy URLs out of the
//! environment, and a [`EnvNoProxy`] struct which parses the no proxy rules from `$no_proxy`.
//! Both structs are also exposed to allow applications to freely combine them with the underlying
//! operating system resolver, however the structure of the rules understood by [`EnvNoProxy`] is
//! hidden because it is subject to change–unfortunately the semantics of `$no_proxy` varies wildly
//! between different libraries and applications, so this crate may receive some updates in that
//! direction in future releases.

use url::{Host, Url};

use crate::ProxyResolver;

/// A single rule for when not to use a proxy.
#[derive(Debug, Clone, PartialEq, Eq)]
enum NoProxyRule {
    MatchExact(String),
    MatchSubdomain(String),
}

impl NoProxyRule {
    fn matches(&self, url: &Url) -> bool {
        match self {
            Self::MatchExact(host) => match url.host() {
                Some(Host::Domain(domain)) => domain == host,
                Some(Host::Ipv4(ipv4)) => &ipv4.to_string() == host,
                Some(Host::Ipv6(ipv6)) => &ipv6.to_string() == host,
                None => false,
            },
            Self::MatchSubdomain(subdomain) => match url.host() {
                Some(Host::Domain(domain)) => {
                    domain.ends_with(subdomain) || domain == &subdomain[1..]
                }
                _ => false,
            },
        }
    }
}

fn lookup(var: &str) -> Option<String> {
    std::env::var_os(var).and_then(|v| {
        v.to_str().map(ToOwned::to_owned).or_else(|| {
            log::warn!("Variable ${} does not contain valid unicode, skipping", var);
            None
        })
    })
}

#[derive(Debug, Clone, PartialEq)]
enum NoProxyRules {
    All,
    Rules(Vec<NoProxyRule>),
}

impl NoProxyRules {
    fn matches(&self, url: &Url) -> bool {
        match self {
            NoProxyRules::All => true,
            NoProxyRules::Rules(ref rules) if rules.iter().any(|rule| rule.matches(url)) => true,
            _ => false,
        }
    }
}

impl From<Vec<NoProxyRule>> for NoProxyRules {
    fn from(rules: Vec<NoProxyRule>) -> Self {
        Self::Rules(rules)
    }
}

/// Simple rules for when not to use a proxy for a given URL.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvNoProxy {
    rules: NoProxyRules,
}

impl EnvNoProxy {
    fn new(rules: NoProxyRules) -> Self {
        Self { rules }
    }

    /// No proxy rules which match no URLs.
    pub fn none() -> Self {
        Self::new(NoProxyRules::Rules(Vec::new()))
    }

    /// No proxy rules which match all URLs.
    pub fn all() -> Self {
        Self::new(NoProxyRules::All)
    }

    /// Parse a curl no proxy rule from `value`.
    ///
    /// See [`Self::from_curl_env()`] for the details of the format.
    pub fn parse_curl_env<S: AsRef<str>>(value: S) -> Self {
        let value = value.as_ref().trim();
        if value == "*" {
            Self::all()
        } else {
            let rules = value
                .split(',')
                .map(|r| r.trim())
                .filter(|r| !r.is_empty())
                .map(|rule| {
                    if rule.starts_with('.') {
                        NoProxyRule::MatchSubdomain(rule.to_string())
                    } else {
                        NoProxyRule::MatchExact(rule.to_string())
                    }
                })
                .collect::<Vec<_>>()
                .into();
            Self::new(rules)
        }
    }

    /// Lookup no proxy rules in Curl environment variables `$no_proxy` and `$NO_PROXY`.
    ///
    /// `$no_proxy` and `$NO_PROXY` either contain a single wildcard `*` or a comma separated list
    /// of hostnames.  In the first case the proxy is disabled for all URLs, in the second case it
    /// is disabled if it matches any hostname in the list.
    ///
    /// If a hostname starts with `.` it matches the host itself as well as all of its subdomains;
    /// otherwise it must match the host exactly.  IPv4 and IPv6 addresses can be used as well, but
    /// are compared as strings, i.e. no wildcards and no subnet specifications.  In other words
    /// neither `192.168.1.*` nor `192.168.1.0/24` will work; there's _no way_ to disable the proxy
    /// for an IP address range.  This limitation is inherted from curl.
    ///
    /// All extra whitespace in rules or around the value is ignored.
    ///
    /// The lowercase `$no_proxy` takes precedence over `$NO_PROXY` if both are defined.
    ///
    /// Return the rules extracted from either variable, or `None` if both variables are unset.
    pub fn from_curl_env() -> Option<Self> {
        lookup("no_proxy")
            .or_else(|| lookup("NO_PROXY"))
            .map(Self::parse_curl_env)
    }

    /// Whether the given `url` matches any no proxy rule.
    ///
    /// In other words returns `true` if the proxy should be disabled for `url`, and `false`
    /// otherwise.
    pub fn matches(&self, url: &Url) -> bool {
        self.rules.matches(url)
    }
}

/// Proxies extracted from the environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvProxies {
    http: Option<Url>,
    https: Option<Url>,
}

impl EnvProxies {
    /// No HTTP and HTTPS proxies in the environment.
    pub fn unset() -> Self {
        Self {
            http: None,
            https: None,
        }
    }

    /// Get proxies defined in the curl environment.
    ///
    /// Get the proxy to use for http and https URLs from `$http_proxy` and `$https_proxy`
    /// respectively.  If one variable is not defined look at the uppercase variants instead;
    /// unlike curl this function also uses `$HTTP_PROXY` as fallback.
    ///
    /// If none of these variables is defined return [`EnvProxies::unset()`].
    ///
    /// See [`curl(1)`](https://curl.se/docs/manpage.html) for details of curl's proxy settings.
    pub fn from_curl_env() -> Self {
        Self {
            http: lookup_url("http_proxy").or_else(|| lookup_url("HTTP_PROXY")),
            https: lookup_url("https_proxy").or_else(|| lookup_url("HTTPS_PROXY")),
        }
    }

    /// Whether no proxies were set in the environment.
    ///
    /// Returns `true` if all of `$http_proxy` and `$https_proxy` as well as their uppercase
    /// variants were not set in the environment.
    pub fn is_unset(&self) -> bool {
        self.http.is_none() && self.https.is_none()
    }

    /// Get the proxy to use for HTTP URLs.
    pub fn http(&self) -> Option<&Url> {
        self.http.as_ref()
    }

    /// Get the proxy to use for HTTPS URLs.
    pub fn https(&self) -> Option<&Url> {
        self.http.as_ref()
    }
}

impl ProxyResolver for EnvProxies {
    fn for_url(&self, url: &Url) -> Option<Url> {
        match url.scheme() {
            "http" => self.http.to_owned(),
            "https" => self.https.to_owned(),
            _ => None,
        }
    }
}

/// Resolve a proxy against a static set of configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvProxyResolver {
    proxies: EnvProxies,
    no_proxy: EnvNoProxy,
}

fn lookup_url(var: &str) -> Option<Url> {
    lookup(var).as_ref().and_then(|s| match Url::parse(s) {
        Ok(url) => Some(url),
        Err(error) => {
            log::warn!(
                "Failed to parse value of ${} as URL, skipping: {}",
                var,
                error
            );
            None
        }
    })
}

impl EnvProxyResolver {
    /// Get proxy rules from environment variables used by curl.
    ///
    /// See [`EnvProxies::from_curl_env()`] and [`EnvNoProxy::from_curl_env()`] for details of
    /// the variables used and their formats.
    ///
    /// This function interprets the environment as does curl, per its documentation, see
    /// [`curl(1)`](https://curl.se/docs/manpage.html).
    ///
    /// `$http_proxy` and `$https_proxy` and their uppercase variants denote the proxy URL for the
    /// given host. The lowercase variant has priority; unlike curl this function also understands
    /// `$HTTP_PROXY`.
    ///
    /// IP addresses are matched as if they were host names, i.e. as strings.  IPv6 addresses
    /// should be given without enclosing brackets.

    /// See [`curl(1)`](https://curl.se/docs/manpage.html) for details of curl's proxy settings.
    pub fn from_curl_env() -> Self {
        let proxies = EnvProxies::from_curl_env();
        let no_proxy = EnvNoProxy::from_curl_env().unwrap_or_else(EnvNoProxy::none);
        Self { proxies, no_proxy }
    }
}

static_assertions::assert_impl_all!(EnvProxyResolver: Send, Sync);

impl ProxyResolver for EnvProxyResolver {
    fn for_url(&self, url: &Url) -> Option<Url> {
        self.proxies.for_url(url).and_then(|proxy| {
            if self.no_proxy.matches(url) {
                None
            } else {
                Some(proxy)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProxyResolver;
    use pretty_assertions::assert_eq;

    #[test]
    fn noproxy_rule_subdomain() {
        let rule = NoProxyRule::MatchSubdomain(".example.com".to_string());
        assert!(rule.matches(&Url::parse("http://example.com/foo").unwrap()));
        assert!(rule.matches(&Url::parse("http://example.com/bar").unwrap()));
        assert!(rule.matches(&Url::parse("http://foo.example.com/foo").unwrap()));
        assert!(!rule.matches(&Url::parse("http://barexample.com/foo").unwrap()));
    }

    #[test]
    fn noproxy_rule_exact_hostname() {
        let rule = NoProxyRule::MatchExact("example.com".to_string());
        assert!(rule.matches(&Url::parse("http://example.com/foo").unwrap()));
        assert!(rule.matches(&Url::parse("http://example.com/bar").unwrap()));
        assert!(!rule.matches(&Url::parse("http://foo.example.com/foo").unwrap()));
        assert!(!rule.matches(&Url::parse("http://barexample.com/foo").unwrap()));
    }

    #[test]
    fn noproxy_rule_exact_ipv4() {
        let rule = NoProxyRule::MatchExact("192.168.100.12".to_string());
        assert!(rule.matches(&Url::parse("http://192.168.100.12/foo").unwrap()));
        assert!(!rule.matches(&Url::parse("http://192.168.100.122/foo").unwrap()));
    }

    #[test]
    fn noproxy_rule_exact_ipv6() {
        let rule = NoProxyRule::MatchExact("fe80::2ead:fea3:1423:6637".to_string());
        assert!(rule.matches(&Url::parse("http://[fe80::2ead:fea3:1423:6637]/foo").unwrap()));
        assert!(!rule.matches(&Url::parse("http://[fe80::2ead:fea3:1423:6638]/foo").unwrap()));
    }

    #[test]
    fn noproxy_rules_all_matches() {
        let samples = vec![
            "http://[fe80::2ead:fea3:1423:6637]/foo",
            "http://192.168.100.12/foo",
            "http://foo.example.com/foo",
            "http:///foo",
        ];
        for url in samples {
            assert!(
                NoProxyRules::All.matches(&Url::parse(url).unwrap()),
                "URL: {}",
                url
            );
        }
    }

    #[test]
    fn noproxy_rules_none_matches() {
        let samples = vec![
            "http://[fe80::2ead:fea3:1423:6637]/foo",
            "http://192.168.100.12/foo",
            "http://foo.example.com/foo",
            "http:///foo",
        ];
        for url in samples {
            assert!(
                !NoProxyRules::Rules(Vec::new()).matches(&Url::parse(url).unwrap()),
                "URL: {}",
                url
            );
        }
    }

    #[test]
    fn noproxy_rules_matches() {
        let rules = NoProxyRules::Rules(vec![
            NoProxyRule::MatchSubdomain(".example.com".to_string()),
            NoProxyRule::MatchExact("192.168.12.100".to_string()),
        ]);

        assert!(rules.matches(&Url::parse("http://example.com").unwrap()));
        assert!(rules.matches(&Url::parse("http://foo.example.com").unwrap()));
        assert!(rules.matches(&Url::parse("http://192.168.12.100/foo").unwrap()));

        assert!(!rules.matches(&Url::parse("http://192.168.12.101/foo").unwrap()));
        assert!(!rules.matches(&Url::parse("http://192.168.12/foo").unwrap()));
        assert!(!rules.matches(&Url::parse("http://fooexample.com/foo").unwrap()));
        assert!(!rules.matches(&Url::parse("http://github.com/swsnr").unwrap()));
    }

    #[test]
    fn from_curl_env_no_env() {
        temp_env::with_vars_unset(
            vec![
                "http_proxy",
                "https_proxy",
                "no_proxy",
                "HTTP_PROXY",
                "HTTPS_PROXY",
                "NO_PROXY",
            ],
            || {
                assert_eq!(
                    EnvProxyResolver::from_curl_env(),
                    EnvProxyResolver {
                        proxies: EnvProxies::unset(),
                        no_proxy: EnvNoProxy::none()
                    }
                )
            },
        )
    }

    #[test]
    fn from_curl_env_lowercase() {
        temp_env::with_vars(
            vec![
                ("http_proxy", Some("http://thehttpproxy:1234")),
                ("https_proxy", Some("http://thehttpsproxy:1234")),
                ("no_proxy", Some("example.com")),
            ],
            || {
                let resolver = EnvProxyResolver::from_curl_env();
                assert_eq!(
                    resolver,
                    EnvProxyResolver {
                        proxies: EnvProxies {
                            http: Some(Url::parse("http://thehttpproxy:1234").unwrap()),
                            https: Some(Url::parse("http://thehttpsproxy:1234").unwrap()),
                        },
                        no_proxy: EnvNoProxy::parse_curl_env("example.com"),
                    }
                )
            },
        )
    }

    #[test]
    fn from_curl_env_uppercase() {
        temp_env::with_vars(
            vec![
                ("http_proxy", None),
                ("https_proxy", None),
                ("no_proxy", None),
                ("HTTP_PROXY", Some("http://thehttpproxy:1234")),
                ("HTTPS_PROXY", Some("http://thehttpsproxy:1234")),
                ("NO_PROXY", Some("example.com")),
            ],
            || {
                let resolver = EnvProxyResolver::from_curl_env();
                assert_eq!(
                    resolver,
                    EnvProxyResolver {
                        proxies: EnvProxies {
                            http: Some(Url::parse("http://thehttpproxy:1234").unwrap()),
                            https: Some(Url::parse("http://thehttpsproxy:1234").unwrap()),
                        },
                        no_proxy: EnvNoProxy::parse_curl_env("example.com"),
                    }
                )
            },
        )
    }

    #[test]
    fn from_curl_env_both() {
        temp_env::with_vars(
            vec![
                ("HTTP_PROXY", Some("http://up.thehttpproxy:1234")),
                ("HTTPS_PROXY", Some("http://up.thehttpsproxy:1234")),
                ("NO_PROXY", Some("up.example.com")),
                ("http_proxy", Some("http://low.thehttpproxy:1234")),
                ("https_proxy", Some("http://low.thehttpsproxy:1234")),
                ("no_proxy", Some("low.example.com")),
            ],
            || {
                let resolver = EnvProxyResolver::from_curl_env();
                assert_eq!(
                    resolver,
                    EnvProxyResolver {
                        proxies: EnvProxies {
                            http: Some(Url::parse("http://low.thehttpproxy:1234").unwrap()),
                            https: Some(Url::parse("http://low.thehttpsproxy:1234").unwrap()),
                        },
                        no_proxy: EnvNoProxy::parse_curl_env("low.example.com"),
                    }
                )
            },
        )
    }

    #[test]
    fn parse_no_proxy_rules_many_rules() {
        let rules = EnvNoProxy::parse_curl_env("example.com ,.example.com , foo.bar,192.122.100.10, fe80::2ead:fea3:1423:6637,[fe80::2ead:fea3:1423:6637]");
        assert_eq!(
            rules.rules,
            NoProxyRules::Rules(vec![
                NoProxyRule::MatchExact("example.com".into()),
                NoProxyRule::MatchSubdomain(".example.com".into()),
                NoProxyRule::MatchExact("foo.bar".into()),
                NoProxyRule::MatchExact("192.122.100.10".into()),
                NoProxyRule::MatchExact("fe80::2ead:fea3:1423:6637".into()),
                NoProxyRule::MatchExact("[fe80::2ead:fea3:1423:6637]".into()),
            ])
        );
    }

    #[test]
    fn parse_no_proxy_rules_wildcard() {
        assert_eq!(EnvNoProxy::parse_curl_env("*"), EnvNoProxy::all());
        assert_eq!(EnvNoProxy::parse_curl_env(" * "), EnvNoProxy::all());
        assert_eq!(
            EnvNoProxy::parse_curl_env("*,foo.example.com").rules,
            NoProxyRules::Rules(vec![
                NoProxyRule::MatchExact("*".into()),
                NoProxyRule::MatchExact("foo.example.com".into())
            ])
        );
    }

    #[test]
    fn parse_no_proxy_rules_empty() {
        assert_eq!(
            EnvNoProxy::parse_curl_env("").rules,
            NoProxyRules::Rules(Vec::new())
        );
        assert_eq!(
            EnvNoProxy::parse_curl_env("  ").rules,
            NoProxyRules::Rules(Vec::new())
        );
        assert_eq!(
            EnvNoProxy::parse_curl_env("\t  ").rules,
            NoProxyRules::Rules(Vec::new())
        );
    }

    #[test]
    fn lookup_http_proxy() {
        let resolver = EnvProxyResolver {
            proxies: EnvProxies {
                http: Some(Url::parse("http://httproxy.example.com:1284").unwrap()),
                https: None,
            },
            no_proxy: EnvNoProxy::none(),
        };
        assert_eq!(
            resolver.for_url(&Url::parse("http://github.com").unwrap()),
            Some(Url::parse("http://httproxy.example.com:1284").unwrap())
        );
        assert_eq!(
            resolver.for_url(&Url::parse("https://github.com").unwrap()),
            None
        );
    }

    #[test]
    fn lookup_https_proxy() {
        let resolver = EnvProxyResolver {
            proxies: EnvProxies {
                http: None,
                https: Some(Url::parse("http://httpsproxy.example.com:1284").unwrap()),
            },
            no_proxy: EnvNoProxy::none(),
        };
        assert_eq!(
            resolver.for_url(&Url::parse("https://github.com").unwrap()),
            Some(Url::parse("http://httpsproxy.example.com:1284").unwrap())
        );
        assert_eq!(
            resolver.for_url(&Url::parse("http://github.com").unwrap()),
            None
        );
    }

    #[test]
    fn lookup_rule_matches() {
        let resolver = EnvProxyResolver {
            proxies: EnvProxies {
                http: Some(Url::parse("http://httproxy.example.com:1284").unwrap()),
                https: Some(Url::parse("http://httpsproxy.example.com:1284").unwrap()),
            },
            no_proxy: EnvNoProxy::all(),
        };
        assert_eq!(
            resolver.for_url(&Url::parse("https://github.com").unwrap()),
            None
        );
        assert_eq!(
            resolver.for_url(&Url::parse("http://github.com").unwrap()),
            None
        );

        let resolver = EnvProxyResolver {
            proxies: EnvProxies {
                http: Some(Url::parse("http://httproxy.example.com:1284").unwrap()),
                https: Some(Url::parse("http://httpsproxy.example.com:1284").unwrap()),
            },
            no_proxy: EnvNoProxy::parse_curl_env("github.com"),
        };
        assert_eq!(
            resolver.for_url(&Url::parse("https://github.com").unwrap()),
            None
        );
        assert_eq!(
            resolver.for_url(&Url::parse("http://github.com").unwrap()),
            None
        );
    }

    #[test]
    fn lookup_rule_does_not_match() {
        let resolver = EnvProxyResolver {
            proxies: EnvProxies {
                http: Some(Url::parse("http://httproxy.example.com:1284").unwrap()),
                https: Some(Url::parse("http://httpsproxy.example.com:1284").unwrap()),
            },
            no_proxy: EnvNoProxy::none(),
        };
        assert_eq!(
            resolver.for_url(&Url::parse("https://github.com").unwrap()),
            Some(Url::parse("http://httpsproxy.example.com:1284").unwrap())
        );
        assert_eq!(
            resolver.for_url(&Url::parse("http://github.com").unwrap()),
            Some(Url::parse("http://httproxy.example.com:1284").unwrap())
        );

        let resolver = EnvProxyResolver {
            proxies: EnvProxies {
                http: Some(Url::parse("http://httproxy.example.com:1284").unwrap()),
                https: Some(Url::parse("http://httpsproxy.example.com:1284").unwrap()),
            },
            no_proxy: EnvNoProxy::parse_curl_env("github.net"),
        };
        assert_eq!(
            resolver.for_url(&Url::parse("https://github.com").unwrap()),
            Some(Url::parse("http://httpsproxy.example.com:1284").unwrap())
        );
        assert_eq!(
            resolver.for_url(&Url::parse("http://github.com").unwrap()),
            Some(Url::parse("http://httproxy.example.com:1284").unwrap())
        );
    }
}
