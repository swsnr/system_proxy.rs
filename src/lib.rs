// Copyright (c) 2022 Sebastian Wiesner <sebastian@swsnr.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![deny(warnings, missing_docs, clippy::all)]

//! Resolve system proxies with various resolvers.
//!
//! Provided resolvers:
//!
//! - [``]
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
//! Windows support is planned, see <https://github.com/swsnr/system_proxy.rs/issues/5>.
//!
//! ## macOS
//!
//! MacOS support may come at some point, see <https://github.com/swsnr/system_proxy.rs/issues/2>.

pub mod env;

#[cfg(all(unix, not(target_os = "mac_os")))]
pub mod unix;
