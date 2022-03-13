// Copyright (c) 2022 Sebastian Wiesner <sebastian@swsnr.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.[cfg(test)]

#![deny(warnings, missing_docs, clippy::all)]

//! Resolve system proxies.
//!
//! TODO: Extensive documentation, samples, and OS specifics.

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
    inner: SystemProxyResolverImpl,
}

impl SystemProxyResolver {
    /// Create a new system proxy resolver.
    pub fn new() -> Self {
        Self {
            inner: SystemProxyResolverImpl::default(),
        }
    }

    /// Resolve system proxy to use for `url`.
    ///
    /// Return the proxy URL to use or `None` for a direct connection.
    ///
    /// # Linux and other unix systems
    ///
    /// On Linux and other Unix systems this function checks Gnome's proxy configuration through
    /// the Gio API, if the corresponding `gio` feature is enabled (default).  This enables support
    /// for more advanced proxy configuration schemes, in particular PAC URLs for proxy
    /// configuration.
    ///
    /// If the `gio` feature is disabled or access to Gnome's proxy configuration failed this
    /// function falls back to the standard environment variables `HTTP_PROXY`, `HTTPS_PROXY` and
    /// `NO_PROXY`, as well as their lower-case variants, through [env_proxy].
    ///
    /// # MacOS
    ///
    /// MacOS is not supported currently.  Pull requests welcome.
    ///
    /// # Windows
    ///
    /// On Windows this function uses the WinHTTP API to resolve the Windows system proxy
    /// configuration.  However it disables automatic resolution of PAC URLs through DHCP or DNS
    /// queries in the local network; this can take several seconds and shouldn't be done
    /// synchronously.  If you require support for this kind of setup please refer to the async
    /// API.
    pub fn for_url(&self, url: &Url) -> Option<Url> {
        self.inner.for_url(url)
    }
}

impl Default for SystemProxyResolver {
    fn default() -> Self {
        Self::new()
    }
}
