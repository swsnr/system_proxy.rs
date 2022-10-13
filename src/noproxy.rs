// Copyright (c) 2022 Sebastian Wiesner <sebastian@swsnr.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/// A proxy resolver which never resolves a proxy.
///
/// Automatically used as fallback proxy resolver if no specific system resolver
/// is enabled at compile time.  In this case [`crate::SystemProxyResolver`]
/// only resolves proxies from the process environment.
#[derive(Default)]
pub struct NoProxyResolver;

impl crate::ProxyResolver for NoProxyResolver {
    fn for_url(&self, _url: &url::Url) -> Option<url::Url> {
        None
    }
}
