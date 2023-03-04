// Copyright (c) 2022 Sebastian Wiesner <sebastian@swsnr.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![deny(warnings, missing_docs, clippy::all)]

//! Lookup HTTP proxies in various ways.
//!
//! ## Available proxy lookup methods
//!
//! - [`env::EnvProxies`] looks up HTTP proxies in the wide-spread `$http_proxy` and
//!   related environment variables.  It aims to be compatible to the well-known `curl` utility.
//! - [`unix::GioProxyResolver`] asynchronously looks up HTTP proxies through Gio and
//!   Glib, i.e. the foundational library of the Gnome desktop environment.  It links against the
//!   Glib library, but in turn supports dynamic proxy configuration, i.e. proxy configuration
//!   which changes while the process is running, and proxy auto-configuration.
//!   This requires the `gio` feature.
//! - [`unix::FreedesktopPortalProxyResolver`] asynchronously looks up HTTP proxies
//!   through the Freedesktop Portal proxy resolver over DBus.  The exact features supported by
//!   this resolver depend on the portal implementation; for Gnome at least the portal has the same
//!   set of features as the Gio resolver.  This resolver does not link against any native
//!   libraries, but in turn requires the [`zbus`] crate for DBus support, and a running portal
//!   implementation at runtime.
//!
//! # Operating system support
//!
//! ## Linux
//!
//! Use either [`unix::GioProxyResolver`] or [`unix::FreedesktopPortalProxyResolver`] to access
//! system proxy settings.
//!
//! ## Windows
//!
//! Windows support is planned, see <https://github.com/swsnr/system_proxy.rs/issues/5>.
//!
//! ## macOS
//!
//! MacOS support may come at some point, see <https://github.com/swsnr/system_proxy.rs/issues/2>.

pub mod env;
pub mod unix;
