// Copyright (c) 2022 Sebastian Wiesner <sebastian@swsnr.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Provide system proxy resolvers for Unix systems (but not MacOS).
//!
//! Notably this module provides the [`crate::unix::gio`] submodule which provides a proxy resolver
//! for the Glib ecosystem, if the `gnome` feature is enabled.
//!
//! It exports the type [`UnixProxyResolver`] as an appropriate default proxy resolver: If the
//! `gnome` feature is enabled it binds to a proxy resolver for the Gio library which in turn uses
//! Gnome's per-user proxy configuration.  Otherwise it binds to a no-op resolver because there is
//! no other source of global proxy configuration on Unix systems; in this case the application can
//! only rely on the proxy environment offered by the [`env`] module.

#[cfg(feature = "gnome")]
pub mod gio;

#[cfg(feature = "gnome")]
pub use self::gio::GioProxyResolver as UnixProxyResolver;

#[cfg(not(feature = "gnome"))]
#[derive(Default)]
/// A proxy resolver which never resolves a proxy.
///
/// Used as a fallback proxy resolver if no specific unix system resolver is enabled at compile
/// time.  In this case [`crate::SystemProxyResolver`] only resolves proxies from the process environment.
pub struct NoProxyResolver;

#[cfg(not(feature = "gnome"))]
impl crate::ProxyResolver for NoProxyResolver {
    fn for_url(&self, _url: &url::Url) -> Option<url::Url> {
        None
    }
}

#[cfg(not(feature = "gnome"))]
pub use NoProxyResolver as UnixProxyResolver;
