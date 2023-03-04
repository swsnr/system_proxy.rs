// Copyright (c) Sebastian Wiesner <sebastian@swsnr.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Provide proxy resolvers for Unix systems.
//!
//! Depending on the enabled features this module provides a Gio based proxy resolver, and/or a
//! resolver using the Freedesktop portal API.

#[cfg(feature = "gio")]
mod gio;
#[cfg(feature = "gio")]
pub use self::gio::GioProxyResolver;

#[cfg(feature = "portal")]
mod portal;
#[cfg(feature = "portal")]
pub use self::portal::FreedesktopPortalProxyResolver;
