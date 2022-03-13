// Copyright (c) 2022 Sebastian Wiesner <sebastian@swsnr.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.[cfg(test)]

//! Resolve proxies via environment variables.

use url::{Host, Url};

#[derive(Debug, Clone, PartialEq)]
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

/// Resolve a proxy against a static set of configuration.
#[allow(warnings)]
#[derive(Debug, PartialEq)]
pub struct EnvProxyResolver {
    http_proxy: Option<Url>,
    https_proxy: Option<Url>,
    no_proxy: NoProxyRules,
}

fn lookup(var: &str) -> Option<String> {
    std::env::var_os(var).and_then(|v| {
        v.to_str().map(ToOwned::to_owned).or_else(|| {
            log::warn!("Variable ${} does not contain valid unicode, skipping", var);
            None
        })
    })
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

fn parse_no_proxy_rules<S: AsRef<str>>(value: S) -> NoProxyRules {
    let value = value.as_ref().trim();
    if value == "*" {
        NoProxyRules::All
    } else {
        value
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
            .into()
    }
}

fn lookup_no_proxy_rules(var: &str) -> Option<NoProxyRules> {
    lookup(var).map(parse_no_proxy_rules)
}

impl EnvProxyResolver {
    /// Get proxy rules from environment variables used by curl.
    ///
    /// This function interprets the environment as does curl, per its documentation, see
    /// [`curl(1)`](https://curl.se/docs/manpage.html).
    ///
    /// `$http_proxy` and `$https_proxy` and their uppercase variants denote the proxy URL for the
    /// given host. The lowercase variant has priority; unlike curl this function also understands
    /// `$HTTP_PROXY`.
    ///
    /// `$no_proxy` and its uppercase variant contain a single wildcard `*` to disable the proxy
    /// for all hosts, or a comma-separate lists of host names.  If a name starts with a dot it
    /// matches the host and all its subdomains.
    ///
    /// IP addresses are matched as if they were host names, i.e. as strings.  IPv6 addresses
    /// should be given without enclosing brackets.
    pub fn from_curl_env() -> Self {
        let http_proxy = lookup_url("http_proxy").or_else(|| lookup_url("HTTP_PROXY"));
        let https_proxy = lookup_url("https_proxy").or_else(|| lookup_url("HTTPS_PROXY"));
        let no_proxy = lookup_no_proxy_rules("no_proxy")
            .or_else(|| lookup_no_proxy_rules("NO_PROXY"))
            .unwrap_or_else(|| NoProxyRules::Rules(Vec::new()));
        Self {
            http_proxy,
            https_proxy,
            no_proxy,
        }
    }
}

#[allow(warnings)]
impl crate::types::ProxyResolver for EnvProxyResolver {
    fn for_url(&self, url: &Url) -> Option<Url> {
        match url.scheme() {
            "http" => self.http_proxy.as_ref(),
            "https" => self.https_proxy.as_ref(),
            _ => None,
        }
        .and_then(|proxy| {
            if self.no_proxy.matches(url) {
                None
            } else {
                Some(proxy.clone())
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
        assert!(!rules.matches(&Url::parse("http://codeberg.org/flausch").unwrap()));
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
                        http_proxy: None,
                        https_proxy: None,
                        no_proxy: NoProxyRules::Rules(Vec::new())
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
                        http_proxy: Some(Url::parse("http://thehttpproxy:1234").unwrap()),
                        https_proxy: Some(Url::parse("http://thehttpsproxy:1234").unwrap()),
                        no_proxy: NoProxyRules::Rules(vec![NoProxyRule::MatchExact(
                            "example.com".to_string()
                        )])
                    }
                )
            },
        )
    }

    #[test]
    fn from_curl_env_uppercase() {
        temp_env::with_vars(
            vec![
                ("HTTP_PROXY", Some("http://thehttpproxy:1234")),
                ("HTTPS_PROXY", Some("http://thehttpsproxy:1234")),
                ("NO_PROXY", Some("example.com")),
            ],
            || {
                let resolver = EnvProxyResolver::from_curl_env();
                assert_eq!(
                    resolver,
                    EnvProxyResolver {
                        http_proxy: Some(Url::parse("http://thehttpproxy:1234").unwrap()),
                        https_proxy: Some(Url::parse("http://thehttpsproxy:1234").unwrap()),
                        no_proxy: NoProxyRules::Rules(vec![NoProxyRule::MatchExact(
                            "example.com".to_string()
                        )])
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
                        http_proxy: Some(Url::parse("http://low.thehttpproxy:1234").unwrap()),
                        https_proxy: Some(Url::parse("http://low.thehttpsproxy:1234").unwrap()),
                        no_proxy: NoProxyRules::Rules(vec![NoProxyRule::MatchExact(
                            "low.example.com".to_string()
                        )])
                    }
                )
            },
        )
    }

    #[test]
    fn parse_no_proxy_rules_many_rules() {
        let rules = parse_no_proxy_rules("example.com ,.example.com , foo.bar,192.122.100.10, fe80::2ead:fea3:1423:6637,[fe80::2ead:fea3:1423:6637]");
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
        assert_eq!(parse_no_proxy_rules("*"), NoProxyRules::All);
        assert_eq!(parse_no_proxy_rules(" * "), NoProxyRules::All);
        assert_eq!(
            parse_no_proxy_rules("*,foo.example.com"),
            NoProxyRules::Rules(vec![
                NoProxyRule::MatchExact("*".into()),
                NoProxyRule::MatchExact("foo.example.com".into())
            ])
        );
    }

    #[test]
    fn parse_no_proxy_rules_empty() {
        assert_eq!(parse_no_proxy_rules(""), NoProxyRules::Rules(Vec::new()));
        assert_eq!(parse_no_proxy_rules("  "), NoProxyRules::Rules(Vec::new()));
        assert_eq!(
            parse_no_proxy_rules("\t  "),
            NoProxyRules::Rules(Vec::new())
        );
    }

    #[test]
    fn lookup_http_proxy() {
        let resolver = EnvProxyResolver {
            http_proxy: Some(Url::parse("http://httproxy.example.com:1284").unwrap()),
            https_proxy: None,
            no_proxy: NoProxyRules::Rules(Vec::new()),
        };
        assert_eq!(
            resolver.for_url(&Url::parse("http://codeberg.org").unwrap()),
            Some(Url::parse("http://httproxy.example.com:1284").unwrap())
        );
        assert_eq!(
            resolver.for_url(&Url::parse("https://codeberg.org").unwrap()),
            None
        );
    }

    #[test]
    fn lookup_https_proxy() {
        let resolver = EnvProxyResolver {
            http_proxy: None,
            https_proxy: Some(Url::parse("http://httpsproxy.example.com:1284").unwrap()),
            no_proxy: NoProxyRules::Rules(Vec::new()),
        };
        assert_eq!(
            resolver.for_url(&Url::parse("https://codeberg.org").unwrap()),
            Some(Url::parse("http://httpsproxy.example.com:1284").unwrap())
        );
        assert_eq!(
            resolver.for_url(&Url::parse("http://codeberg.org").unwrap()),
            None
        );
    }

    #[test]
    fn lookup_rule_matches() {
        let resolver = EnvProxyResolver {
            http_proxy: Some(Url::parse("http://httproxy.example.com:1284").unwrap()),
            https_proxy: None,
            no_proxy: NoProxyRules::All,
        };
        assert_eq!(
            resolver.for_url(&Url::parse("https://codeberg.org").unwrap()),
            None
        );
        assert_eq!(
            resolver.for_url(&Url::parse("http://codeberg.org").unwrap()),
            None
        );

        let resolver = EnvProxyResolver {
            http_proxy: Some(Url::parse("http://httproxy.example.com:1284").unwrap()),
            https_proxy: None,
            no_proxy: NoProxyRules::Rules(vec![NoProxyRule::MatchExact("codeberg.org".into())]),
        };
        assert_eq!(
            resolver.for_url(&Url::parse("https://codeberg.org").unwrap()),
            None
        );
        assert_eq!(
            resolver.for_url(&Url::parse("http://codeberg.org").unwrap()),
            None
        );
    }

    #[test]
    fn lookup_rule_does_not_match() {
        let resolver = EnvProxyResolver {
            http_proxy: Some(Url::parse("http://httproxy.example.com:1284").unwrap()),
            https_proxy: Some(Url::parse("http://httpsproxy.example.com:1284").unwrap()),
            no_proxy: NoProxyRules::Rules(Vec::new()),
        };
        assert_eq!(
            resolver.for_url(&Url::parse("https://codeberg.org").unwrap()),
            Some(Url::parse("http://httpsproxy.example.com:1284").unwrap())
        );
        assert_eq!(
            resolver.for_url(&Url::parse("http://codeberg.org").unwrap()),
            Some(Url::parse("http://httproxy.example.com:1284").unwrap())
        );

        let resolver = EnvProxyResolver {
            http_proxy: Some(Url::parse("http://httproxy.example.com:1284").unwrap()),
            https_proxy: Some(Url::parse("http://httpsproxy.example.com:1284").unwrap()),
            no_proxy: NoProxyRules::Rules(vec![NoProxyRule::MatchExact("github.com".into())]),
        };
        assert_eq!(
            resolver.for_url(&Url::parse("https://codeberg.org").unwrap()),
            Some(Url::parse("http://httpsproxy.example.com:1284").unwrap())
        );
        assert_eq!(
            resolver.for_url(&Url::parse("http://codeberg.org").unwrap()),
            Some(Url::parse("http://httproxy.example.com:1284").unwrap())
        );
    }
}
