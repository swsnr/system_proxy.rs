// Copyright (c) 2022 Sebastian Wiesner <sebastian@swsnr.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![deny(warnings, missing_docs, clippy::all)]

//! Resolve system proxies.
//!
//! # Simple usage
//!
//! The [`default()`] function returns the default proxy resolver of the operating system, wrapped
//! with a [`env::EnvProxyResolver`] which looks at the standard `$HTTP_PROXY` and friends used by
//! most command line utilities:
//!
//! ```
//! let proxy = system_proxy::default();
//! let client = reqwest::blocking::Client::builder()
//!     .user_agent(concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")))
//!     .proxy(reqwest::Proxy::custom(move |u| proxy.for_url(u)))
//!     .build()
//!     .unwrap();
//!
//! let response = client.get("https://httpbin.org/status/200").send().unwrap();
//! println!("Status code: {}", response.status());
//! ```
//!
//! ## Proxy resolvers
//!
//! A [`ProxyResolver`] provides the [`ProxyResolver::for_url()`] method which returns the HTTP
//! proxy URL to use for the given URL, Or `None` if a direct connection should be used.
//!
//! # Advanced usage
//!
//! This crate also offers direct access to the operating system proxy resolver, via exported
//! modules such as [`unix`].  It also offers direct access to a resolver which uses the standard
//! `$HTTP_PROXY` etc. environment variables, via [`env::EnvProxyResolver`].  The corresponding
//! module also exposes the the components of the proxy resolver, namely access to the proxy
//! variables as well as to `$NO_PROXY` rules, which allows for flexible composition of proxy
//! rules.
//!
//! # Operating system support
//!
//! In addition to environment variables this crate mainly exposes the default proxy resolver of
//! the underlying operating system.
//!
//! ## Linux and other Unix systems
//!
//! Linux and other related Unix systems such as FreeBSD do not offer a standard system-wide HTTP
//! proxy resolver.  Most unix programs use the wide-spread `$HTTP_PROXY` etc. environment
//! variables.
//!
//! However these variables suffer from various drawbacks: Applications using these variables cannot
//! dynamically react on changes to the network connection, e.g. disconnecting from a public Wifi
//! and connecting to the company ethernet, each using a different proxy.  Instead the application
//! needs to restart to obtain new values for the proxy configuration, which often requires users
//! to restart their entire session to get an updated proxy environment.  These applications also
//! lack support for more sophisticated proxy configuration schemes, namely auto-configuration
//! URLs, which are widely used in enterprise environments.
//!
//! For this reason Gnome at least offers a separate per-user proxy configuration which allows to
//! change the proxy dynamically for all running applications, and adds support for
//! auto-configuration URLs.  Most Glib/Gio based applications use this infrastructure.
//!
//! This crate binds to Gio and uses the Gio proxy resolver to make use of this Gnome-wide proxy
//! configuration.
//!
//! ## Windows
//!
//! Windows support is planned, see <https://github.com/lunaryorn/system_proxy.rs/issues/5>.
//!
//! ## macOS
//!
//! MacOS support may come at some point, see <https://github.com/lunaryorn/system_proxy.rs/issues/2>.
//!
//! # Async API
//!
//! An async API is planned, see <https://github.com/lunaryorn/system_proxy.rs/issues/3>, however
//! most HTTP clients such as reqwest only offer a synchronous API to set the HTTP proxy, so this
//! has somewhat lower priority.

use env::{EnvNoProxy, EnvProxies};
use url::Url;

mod types;

pub mod env;

#[cfg(all(unix, not(target_os = "mac_os")))]
pub mod unix;

pub use types::ProxyResolver;

#[cfg(all(unix, not(target_os = "mac_os")))]
use unix::UnixProxyResolver as SystemProxyResolverImpl;

/// The system proxy resolver.
///
/// Resolve proxies from system configuration, and through operating system APIs.
pub struct SystemProxyResolver {
    env_proxies: EnvProxies,
    env_no_proxy: EnvNoProxy,
    system: SystemProxyResolverImpl,
}

impl SystemProxyResolver {
    /// Create a new system proxy resolver.
    ///
    /// Creates an instance of the standard proxy resolver for the current operating system and
    /// uses the given environment proxy settings.
    pub fn new(env_proxies: EnvProxies, env_no_proxy: EnvNoProxy) -> Self {
        Self {
            env_proxies,
            env_no_proxy,
            system: SystemProxyResolverImpl::default(),
        }
    }

    /// Create a system proxy resolver which never looks at the environment.
    ///
    /// This resolver will only use the standard proxy resolver of the operating system.
    pub fn no_env() -> Self {
        Self::new(EnvProxies::unset(), EnvNoProxy::none())
    }

    /// Resolve system proxy to use for `url`.
    ///
    /// Return the proxy URL to use or `None` for a direct connection.
    ///
    /// # Environment
    ///
    /// On all systems this resolver first looks at the proxy settings in the environment, per
    /// [`env::EnvProxies`] and [`env::EnvNoProxy`].
    ///
    /// The proxies specified in the environment take precedence over the system proxy, and the
    /// system proxy is not consulted for URLs that match the [`env::EnvNoProxy`] settings.
    /// This allows to quickly disable all proxying for a given application by setting `$no_proxy`
    /// to `*`, even if the appliation looks up a system proxy.
    ///
    /// # Operating system proxy resolver
    ///
    /// ## Linux and other unix systems
    ///
    /// On Linux and other Unix systems this function checks Gnome's proxy configuration through
    /// the Gio API, if the corresponding `gio` feature is enabled (default).  This enables support
    /// for more advanced proxy configuration schemes, in particular PAC URLs for proxy
    /// configuration.
    ///
    /// ## MacOS
    ///
    /// MacOS is not supported currently.  Pull requests welcome.
    ///
    /// ## Windows
    ///
    /// On Windows this function uses the WinHTTP API to resolve the Windows system proxy
    /// configuration.  However it disables automatic resolution of PAC URLs through DHCP or DNS
    /// queries in the local network; this can take several seconds and shouldn't be done
    /// synchronously.  If you require support for this kind of setup please refer to the async
    /// API.
    pub fn for_url(&self, url: &Url) -> Option<Url> {
        if self.env_no_proxy.matches(url) {
            None
        } else {
            self.env_proxies
                .for_url(url)
                .or_else(|| self.system.for_url(url))
        }
    }
}

static_assertions::assert_impl_all!(SystemProxyResolver: Send, Sync);

impl Default for SystemProxyResolver {
    /// Create the default system proxy resolver.
    ///
    /// This proxy resolver uses the curl environment by default (see
    /// [`env::EnvProxies::from_curl_env()`] and [`env::EnvNoProxy::from_curl_env()`]) by default
    /// and falls back to the standard proxy resolver of the operating system.
    ///
    /// See [`SystemProxyResolver::for_url`] for more information about how the proxy is resolved.
    fn default() -> Self {
        Self::new(
            EnvProxies::from_curl_env(),
            EnvNoProxy::from_curl_env().unwrap_or_else(EnvNoProxy::none),
        )
    }
}

/// Return the default system proxy resolver, i.e. [`SystemProxyResolver::default()`].
pub fn default() -> SystemProxyResolver {
    SystemProxyResolver::default()
}
