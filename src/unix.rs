// Copyright (c) 2022 Sebastian Wiesner <sebastian@swsnr.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Provide proxy resolvers for Unix (non-macOS) systems.
//!
//! Depending on the enabled features this module provides a Gio based proxy resolver.

#[cfg(feature = "gio")]
pub mod gio;
