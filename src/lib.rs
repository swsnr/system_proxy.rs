// Copyright (c) 2022 Sebastian Wiesner <sebastian@swsnr.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.[cfg(test)]

#![deny(warnings, missing_docs, clippy::all)]

//! Resolve system proxies.
//!
//! TODO: Extensive documentation, samples, and OS specifics.

use env::{EnvNoProxy, EnvProxies};
use url::Url;

mod noproxy;
mod types;

pub mod env;

#[cfg(all(unix, not(target_os = "mac_os")))]
pub mod unix;

pub use noproxy::NoProxy;
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

impl Default for SystemProxyResolver {
    fn default() -> Self {
        Self::new(
            EnvProxies::from_curl_env(),
            EnvNoProxy::from_curl_env().unwrap_or_else(EnvNoProxy::none),
        )
    }
}
