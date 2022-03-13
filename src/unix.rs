// Copyright (c) 2022 Sebastian Wiesner <sebastian@swsnr.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.[cfg(test)]

//! Provide system proxy resolvers for Unix systems (but not MacOS).
//!
//! Notably this module provides the [`crate::unix::gio`] submodule which provides a proxy resolver
//! for the Glib ecosystem, if the `gnome` feature is enabled.
//!
//! It exports the type [`UnixProxyResolver`] as an appropriate default proxy resolver.

#[cfg(feature = "gnome")]
pub mod gio;

#[cfg(feature = "gnome")]
pub use self::gio::GioProxyResolver as UnixProxyResolver;

#[cfg(not(feature = "gnome"))]
pub use crate::noproxy::NoProxy as UnixProxyResolver;
