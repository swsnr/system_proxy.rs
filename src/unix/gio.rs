// Copyright (c) 2022 Sebastian Wiesner <sebastian@swsnr.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Get system proxy from Gio, that is, Gnome system settings.
//!
//! This proxy resolver supports all features of Gnome system settings, including PAC URLs.
//!
//! This module requires the `gnome` feature which is enabled by default.

use gio::glib;
use gio::traits::ProxyResolverExt;
use log::{debug, error};
use url::Url;

/// A proxy resolver for Gio and Glib.
///
/// This resolver uses configuration from GSettings, i.e. Gnome configuration.  Depending on how
/// Gnome is set up it supports simple proxy settings as well as PAC URLs.
pub struct GioProxyResolver;

impl GioProxyResolver {
    /// Lookup the Gio proxy for the given URL.
    ///
    /// Return the proxy to use, or `None` for a direct connection.  If accessing the proxy
    /// configuration fails or the proxy configuration returns an invalid URL return the
    /// corresponding error.
    pub fn lookup(&self, url: &Url) -> Result<Option<Url>, glib::Error> {
        // We always construct a new proxy resolver per call, because gojects and thus
        // gio::ProxyResolver are not thread-thread safe, so this struct wouldn't be Send + Sync.
        let proxies = gio::ProxyResolver::default().lookup(url.as_str(), gio::Cancellable::NONE)?;
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

static_assertions::assert_impl_all!(GioProxyResolver: Send, Sync);

impl crate::types::ProxyResolver for GioProxyResolver {
    fn for_url(&self, url: &Url) -> Option<Url> {
        self.lookup(url)
            .unwrap_or_else(|error| {
                error!("Failed to obtain proxy for URL {}: {}", url, error);
                None
            })
            .map(|proxy| {
                debug!("Obtained proxy {:?} for URL {} from Gio", proxy, url);
                proxy
            })
    }
}

impl Default for GioProxyResolver {
    fn default() -> Self {
        Self
    }
}
