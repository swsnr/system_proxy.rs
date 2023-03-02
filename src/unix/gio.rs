// Copyright (c) 2022 Sebastian Wiesner <sebastian@swsnr.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Provide a Gio proxy resolver.
//!
//! This module prpvides a thin wrapper around [`Gio.ProxyResolver`](https://docs.gtk.org/gio/iface.ProxyResolver.html)
//! from Glib/Gio, and adds a more convenient [`Url`]-based API around the underlying API.
//!
//! This module requires the `gio` feature.

use gio::glib;
use gio::traits::ProxyResolverExt;
use url::Url;

/// A convenience wrapper around [`gio::ProxyResolver`].
///
/// See [`Gio.ProxyResolver`](https://docs.gtk.org/gio/iface.ProxyResolver.html) for the underlying
/// Gio type.
///
/// This type can be cloned cheaply.
#[derive(Debug, Clone)]
pub struct GioProxyResolver {
    resolver: gio::ProxyResolver,
}

impl GioProxyResolver {
    /// Wrap the given GIO proxy `resolver`.
    pub fn new(resolver: gio::ProxyResolver) -> Self {
        Self { resolver }
    }

    /// Lookup the Gio proxy for the given `url`.
    ///
    /// Return the proxy to use, or `None` for a direct connection.  If accessing the proxy
    /// configuration fails or the proxy configuration returns an invalid URL return the
    /// corresponding error.
    pub async fn lookup(&self, url: &Url) -> Result<Option<Url>, glib::Error> {
        let proxies = self.resolver.lookup_future(url.as_str()).await?;
        match proxies.get(0) {
            None => Ok(None),
            Some(url) if url == "direct://" => Ok(None),
            Some(url) => Url::parse(url).map(Some).map_err(|parse_error| {
                glib::Error::new(
                    glib::UriError::Failed,
                    &format!("Failed to parse proxy URL {}: {}", url, parse_error),
                )
            }),
        }
    }
}

impl Default for GioProxyResolver {
    /// Get the default proxy resolver.
    ///
    /// See [`gio::ProxyResolver::default`], and [`g_proxy_resolver_get_default`](https://docs.gtk.org/gio/type_func.ProxyResolver.get_default.htmll)
    /// for the underlying Gio function.
    fn default() -> Self {
        Self {
            resolver: gio::ProxyResolver::default(),
        }
    }
}
