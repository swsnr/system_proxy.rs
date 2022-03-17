// Copyright (c) 2022 Sebastian Wiesner <sebastian@swsnr.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use url::Url;

/// Resolve proxies.
pub trait ProxyResolver {
    /// Resolve a proxy for the given `url`.
    ///
    /// Return the URL of a HTTP proxy to use for `url` or `None` for a direct connection to `url`.
    fn for_url(&self, url: &Url) -> Option<Url>;
}
