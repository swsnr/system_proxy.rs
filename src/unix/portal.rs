// Copyright (c) Sebastian Wiesner <sebastian@swsnr.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Lookup proxies on the [Freedesktop Proxy Portal](https://flatpak.github.io/xdg-desktop-portal/#gdbus-org.freedesktop.portal.ProxyResolver).
//!
//! Similar to the GIO resolver, but does not require a Glib/GIO dependency.  Instead it uses zbus
//! to talk to the DBus service directly.
//!
//! This module requires the `portal` feature.

use url::Url;
use zbus::{Connection, Result};

/// A proxy resolver which uses the Freedesktop proxy resolver portal.
///
/// This struct only holds the underlying [`zbus::Connection`]; consequently it's cheap to clone
/// this struct.
#[derive(Debug, Clone)]
pub struct FreedesktopPortalProxyResolver {
    connection: zbus::Connection,
}

static_assertions::assert_impl_all!(FreedesktopPortalProxyResolver: Send, Sync);

impl<'a> FreedesktopPortalProxyResolver {
    /// Use the proxy resolver portal on the given `connection`.
    pub fn new(connection: Connection) -> Self {
        Self { connection }
    }

    /// Connect to session bus and use its proxy resolver portal.
    pub async fn connect() -> Result<Self> {
        Ok(Self::new(zbus::Connection::session().await?))
    }

    /// Lookup the proxy for the given `url`.
    ///
    /// Return the proxy to use, or `None` for a direct connection.  If accessing the proxy
    /// resolver portal failed or the connection to DBus died, return the corresponding error.
    pub async fn lookup(&self, url: &Url) -> Result<Option<Url>> {
        let proxies: Vec<String> = self
            .connection
            .call_method(
                Some("org.freedesktop.portal.Desktop"),
                "/org/freedesktop/portal/desktop",
                Some("org.freedesktop.portal.ProxyResolver"),
                "Lookup",
                &(url.as_str(),),
            )
            .await?
            .body()?;

        match proxies.get(0) {
            None => Ok(None),
            Some(url) if url == "direct://" => Ok(None),
            Some(url) => Url::parse(url).map(Some).map_err(|parse_error| {
                zbus::Error::Failure(format!("Failed to parse proxy URL {url}: {parse_error}",))
            }),
        }
    }
}
