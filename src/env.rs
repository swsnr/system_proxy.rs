// Copyright (c) 2022 Sebastian Wiesner <sebastian@swsnr.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Resolve proxies via environment variables.
//!
//! This module provides means to get proxy settings from the environment as understood by the
//! [curl](https://curl.se/) tool.
//!
//! The [`EnvProxies`] struct extracts the HTTP and HTTPS proxies as well as no-proxy rules from
//! the curl environment variables (see [`EnvProxies::from_curl_env`]).  The latter part is
//! available separately via [`NoProxyRules`].
//!
//! Note that the precise meaning of no-proxy rules in the relevant environment variables varies
//! wildly between different implementations.  This module tries to follow curl as closely as
//! possible for maximum compatibility, and thus does not support more advanced no-proxy rules,
//! e.g. based on IP subnet masks.

use std::ops::Not;

use url::{Host, Url};

/// A trait which represents a rule for when to skip a proxy.
pub trait NoProxy {
    /// Whether *not* to use a proxy for the given `url`.
    ///
    /// Return `true` if a direct connection should be used for `url`, or `false` if `url` should
    /// use a proxy.
    fn no_proxy_for(&self, url: &Url) -> bool;

    /// Whether to use a proxy for the given `url`.
    ///
    /// Return `true` if a proxy should be used for `url` or `false` if a direct connection should
    /// be used.
    fn proxy_allowed_for(&self, url: &Url) -> bool {
        self.no_proxy_for(url).not()
    }
}

/// A single rule for when not to use a proxy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoProxyRule {
    /// Match the given hostname exactly.
    MatchExact(String),
    /// Match a domain and all its subdomains.
    MatchSubdomain(String),
}

static_assertions::assert_impl_all!(NoProxyRule: Send, Sync);

impl NoProxy for NoProxyRule {
    fn no_proxy_for(&self, url: &Url) -> bool {
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

/// Combine multiple rules for when not to use a proxy.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum NoProxyRules {
    /// Do not use a proxy for all hosts.
    All,
    /// Do not use a proxy if any of the given rules matches.
    ///
    /// If the list of rules is empty, always use a proxy.
    Rules(Vec<NoProxyRule>),
}

static_assertions::assert_impl_all!(NoProxyRules: Send, Sync);

fn lookup(var: &str) -> Option<String> {
    std::env::var_os(var).and_then(|v| {
        v.to_str().map(ToOwned::to_owned).or_else(|| {
            log::warn!("Variable ${} does not contain valid unicode, skipping", var);
            None
        })
    })
}

impl NoProxyRules {
    /// Create no proxy rules.
    pub fn new(rules: Vec<NoProxyRule>) -> Self {
        Self::Rules(rules)
    }

    /// Use a proxy for all URLs.
    pub fn none() -> Self {
        NoProxyRules::Rules(Vec::new())
    }

    /// Never use a proxy for any URL.
    pub fn all() -> Self {
        Self::All
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
                .collect::<Vec<_>>();
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
}

impl NoProxy for NoProxyRules {
    fn no_proxy_for(&self, url: &Url) -> bool {
        match self {
            NoProxyRules::All => true,
            NoProxyRules::Rules(ref rules) => rules.iter().any(|rule| rule.no_proxy_for(url)),
        }
    }
}

impl From<Vec<NoProxyRule>> for NoProxyRules {
    fn from(rules: Vec<NoProxyRule>) -> Self {
        Self::new(rules)
    }
}

impl From<NoProxyRule> for NoProxyRules {
    fn from(rule: NoProxyRule) -> Self {
        Self::new(vec![rule])
    }
}

impl Default for NoProxyRules {
    /// Empty no proxy rules, i.e. always use a proxy.
    fn default() -> Self {
        NoProxyRules::none()
    }
}

/// Proxies extracted from the environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvProxies {
    /// The proxy to use for `http:` URLs.
    ///
    /// `None` if no HTTP proxy was set in the environment.
    pub http: Option<Url>,
    /// The proxy to use for `https:` URLs.
    ///
    /// `None` if no HTTPS proxy was set in the environment.
    pub https: Option<Url>,
    /// When not to use a proxy.
    ///
    /// `None` if no such rules where present in the environment.
    pub no_proxy_rules: Option<NoProxyRules>,
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

impl EnvProxies {
    /// No HTTP and HTTPS proxies in the environment.
    pub fn unset() -> Self {
        Self {
            http: None,
            https: None,
            no_proxy_rules: None,
        }
    }

    /// Get proxies defined in the curl environment.
    ///
    /// Get the proxy to use for http and https URLs from `$http_proxy` and `$https_proxy`
    /// respectively.  If one variable is not defined look at the uppercase variants instead;
    /// unlike curl this function also uses `$HTTP_PROXY` as fallback.
    ///
    /// IP addresses are matched as if they were host names, i.e. as strings.  IPv6 addresses
    /// should be given without enclosing brackets.
    ///
    /// If either of these proxies is set also look take no proxy rules from the curl environemnt
    /// with [`NoProxyRules::from_curl_env()`]
    ///
    /// If none of these variables is defined return [`EnvProxies::unset()`].
    ///
    /// See [`curl(1)`](https://curl.se/docs/manpage.html) for details of curl's proxy settings.
    pub fn from_curl_env() -> Self {
        Self {
            http: lookup_url("http_proxy").or_else(|| lookup_url("HTTP_PROXY")),
            https: lookup_url("https_proxy").or_else(|| lookup_url("HTTPS_PROXY")),
            no_proxy_rules: NoProxyRules::from_curl_env(),
        }
    }

    /// Whether no proxies were set in the environment.
    ///
    /// Returns `true` if all of `$http_proxy` and `$https_proxy` as well as their uppercase
    /// variants were not set in the environment.
    pub fn is_unset(&self) -> bool {
        self.http.is_none() && self.https.is_none()
    }

    /// Lookup a proxy server for the given `url`.
    pub fn lookup(&self, url: &Url) -> Option<&Url> {
        let rules = self.no_proxy_rules.as_ref();
        let proxy = match url.scheme() {
            "http" => self.http.as_ref(),
            "https" => self.https.as_ref(),
            _ => None,
        };
        if proxy.is_some() && rules.map_or(true, |r| r.proxy_allowed_for(url)) {
            proxy
        } else {
            None
        }
    }
}

/// Get proxies from curl environment.
///
/// See [`EnvProxies::from_curl_env`].
pub fn from_curl_env() -> EnvProxies {
    EnvProxies::from_curl_env()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn noproxy_rule_subdomain() {
        let rule = NoProxyRule::MatchSubdomain(".example.com".to_string());
        assert!(rule.no_proxy_for(&Url::parse("http://example.com/foo").unwrap()));
        assert!(rule.no_proxy_for(&Url::parse("http://example.com/bar").unwrap()));
        assert!(rule.no_proxy_for(&Url::parse("http://foo.example.com/foo").unwrap()));
        assert!(!rule.no_proxy_for(&Url::parse("http://barexample.com/foo").unwrap()));
    }

    #[test]
    fn noproxy_rule_exact_hostname() {
        let rule = NoProxyRule::MatchExact("example.com".to_string());
        assert!(rule.no_proxy_for(&Url::parse("http://example.com/foo").unwrap()));
        assert!(rule.no_proxy_for(&Url::parse("http://example.com/bar").unwrap()));
        assert!(!rule.no_proxy_for(&Url::parse("http://foo.example.com/foo").unwrap()));
        assert!(!rule.no_proxy_for(&Url::parse("http://barexample.com/foo").unwrap()));
    }

    #[test]
    fn noproxy_rule_exact_ipv4() {
        let rule = NoProxyRule::MatchExact("192.168.100.12".to_string());
        assert!(rule.no_proxy_for(&Url::parse("http://192.168.100.12/foo").unwrap()));
        assert!(!rule.no_proxy_for(&Url::parse("http://192.168.100.122/foo").unwrap()));
    }

    #[test]
    fn noproxy_rule_exact_ipv6() {
        let rule = NoProxyRule::MatchExact("fe80::2ead:fea3:1423:6637".to_string());
        assert!(rule.no_proxy_for(&Url::parse("http://[fe80::2ead:fea3:1423:6637]/foo").unwrap()));
        assert!(!rule.no_proxy_for(&Url::parse("http://[fe80::2ead:fea3:1423:6638]/foo").unwrap()));
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
                NoProxyRules::All.no_proxy_for(&Url::parse(url).unwrap()),
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
                !NoProxyRules::Rules(Vec::new()).no_proxy_for(&Url::parse(url).unwrap()),
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

        assert!(rules.no_proxy_for(&Url::parse("http://example.com").unwrap()));
        assert!(rules.no_proxy_for(&Url::parse("http://foo.example.com").unwrap()));
        assert!(rules.no_proxy_for(&Url::parse("http://192.168.12.100/foo").unwrap()));

        assert!(!rules.no_proxy_for(&Url::parse("http://192.168.12.101/foo").unwrap()));
        assert!(!rules.no_proxy_for(&Url::parse("http://192.168.12/foo").unwrap()));
        assert!(!rules.no_proxy_for(&Url::parse("http://fooexample.com/foo").unwrap()));
        assert!(!rules.no_proxy_for(&Url::parse("http://github.com/swsnr").unwrap()));
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
                    EnvProxies::from_curl_env(),
                    EnvProxies {
                        http: None,
                        https: None,
                        no_proxy_rules: None
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
                assert_eq!(
                    EnvProxies::from_curl_env(),
                    EnvProxies {
                        http: Some(Url::parse("http://thehttpproxy:1234").unwrap()),
                        https: Some(Url::parse("http://thehttpsproxy:1234").unwrap()),
                        no_proxy_rules: Some(
                            NoProxyRule::MatchExact("example.com".to_string()).into()
                        )
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
                assert_eq!(
                    EnvProxies::from_curl_env(),
                    EnvProxies {
                        http: Some(Url::parse("http://thehttpproxy:1234").unwrap()),
                        https: Some(Url::parse("http://thehttpsproxy:1234").unwrap()),
                        no_proxy_rules: Some(
                            NoProxyRule::MatchExact("example.com".to_string()).into()
                        )
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
                assert_eq!(
                    EnvProxies::from_curl_env(),
                    EnvProxies {
                        http: Some(Url::parse("http://low.thehttpproxy:1234").unwrap()),
                        https: Some(Url::parse("http://low.thehttpsproxy:1234").unwrap()),
                        no_proxy_rules: Some(
                            NoProxyRule::MatchExact("low.example.com".to_string()).into()
                        )
                    }
                )
            },
        )
    }

    #[test]
    fn parse_no_proxy_rules_many_rules() {
        let rules = NoProxyRules::parse_curl_env("example.com ,.example.com , foo.bar,192.122.100.10, fe80::2ead:fea3:1423:6637,[fe80::2ead:fea3:1423:6637]");
        assert_eq!(
            rules,
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
        assert_eq!(NoProxyRules::parse_curl_env("*"), NoProxyRules::all());
        assert_eq!(NoProxyRules::parse_curl_env(" * "), NoProxyRules::all());
        assert_eq!(
            NoProxyRules::parse_curl_env("*,foo.example.com"),
            NoProxyRules::Rules(vec![
                NoProxyRule::MatchExact("*".into()),
                NoProxyRule::MatchExact("foo.example.com".into())
            ])
        );
    }

    #[test]
    fn parse_no_proxy_rules_empty() {
        assert_eq!(NoProxyRules::parse_curl_env(""), NoProxyRules::default());
        assert_eq!(NoProxyRules::parse_curl_env("  "), NoProxyRules::default());
        assert_eq!(
            NoProxyRules::parse_curl_env("\t  "),
            NoProxyRules::default()
        );
    }

    #[test]
    fn lookup_http_proxy() {
        let proxies = EnvProxies {
            http: Some(Url::parse("http://httproxy.example.com:1284").unwrap()),
            https: None,
            no_proxy_rules: Some(NoProxyRules::default()),
        };
        assert_eq!(
            proxies.lookup(&Url::parse("http://github.com").unwrap()),
            Some(&Url::parse("http://httproxy.example.com:1284").unwrap())
        );
        assert_eq!(
            proxies.lookup(&Url::parse("https://github.com").unwrap()),
            None
        );
    }

    #[test]
    fn lookup_https_proxy() {
        let proxies = EnvProxies {
            http: None,
            https: Some(Url::parse("http://httpsproxy.example.com:1284").unwrap()),
            no_proxy_rules: Some(NoProxyRules::default()),
        };
        assert_eq!(
            proxies.lookup(&Url::parse("https://github.com").unwrap()),
            Some(&Url::parse("http://httpsproxy.example.com:1284").unwrap())
        );
        assert_eq!(
            proxies.lookup(&Url::parse("http://github.com").unwrap()),
            None
        );
    }

    #[test]
    fn lookup_rule_matches() {
        let proxies = EnvProxies {
            http: Some(Url::parse("http://httproxy.example.com:1284").unwrap()),
            https: Some(Url::parse("http://httpsproxy.example.com:1284").unwrap()),
            no_proxy_rules: Some(NoProxyRules::All),
        };
        assert_eq!(
            proxies.lookup(&Url::parse("https://github.com").unwrap()),
            None
        );
        assert_eq!(
            proxies.lookup(&Url::parse("http://github.com").unwrap()),
            None
        );

        let proxies = EnvProxies {
            http: Some(Url::parse("http://httproxy.example.com:1284").unwrap()),
            https: Some(Url::parse("http://httpsproxy.example.com:1284").unwrap()),
            no_proxy_rules: Some(NoProxyRules::parse_curl_env("github.com")),
        };
        assert_eq!(
            proxies.lookup(&Url::parse("https://github.com").unwrap()),
            None
        );
        assert_eq!(
            proxies.lookup(&Url::parse("http://github.com").unwrap()),
            None
        );
    }

    #[test]
    fn lookup_rule_does_not_match() {
        let resolver = EnvProxies {
            http: Some(Url::parse("http://httproxy.example.com:1284").unwrap()),
            https: Some(Url::parse("http://httpsproxy.example.com:1284").unwrap()),
            no_proxy_rules: Some(NoProxyRules::default()),
        };
        assert_eq!(
            resolver.lookup(&Url::parse("https://github.com").unwrap()),
            Some(&Url::parse("http://httpsproxy.example.com:1284").unwrap())
        );
        assert_eq!(
            resolver.lookup(&Url::parse("http://github.com").unwrap()),
            Some(&Url::parse("http://httproxy.example.com:1284").unwrap())
        );

        let proxies = EnvProxies {
            http: Some(Url::parse("http://httproxy.example.com:1284").unwrap()),
            https: Some(Url::parse("http://httpsproxy.example.com:1284").unwrap()),
            no_proxy_rules: Some(NoProxyRules::parse_curl_env("github.net")),
        };
        assert_eq!(
            proxies.lookup(&Url::parse("https://github.com").unwrap()),
            Some(&Url::parse("http://httpsproxy.example.com:1284").unwrap())
        );
        assert_eq!(
            proxies.lookup(&Url::parse("http://github.com").unwrap()),
            Some(&Url::parse("http://httproxy.example.com:1284").unwrap())
        );
    }
}
