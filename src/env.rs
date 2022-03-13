// Copyright (c) 2022 Sebastian Wiesner <sebastian@swsnr.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.[cfg(test)]

//! Resolve proxies via environment variables.

use url::Url;

/// Resolve a proxy against a static set of configuration.
#[allow(warnings)]
pub struct EnvProxyResolver {
    http_proxy: Option<Url>,
    https_proxy: Option<Url>,
    // TODO: Perhaps parse this into a proper set of rules?
    no_proxy: Vec<String>,
}

impl EnvProxyResolver {
    /// Get proxy rules from environment variables used by curl.
    ///
    /// See https://github.com/curl/curl/issues/1208
    /// TODO: Document!
    pub fn from_curl_env() -> Self {
        todo!()
    }
}

#[allow(warnings)]
impl crate::types::ProxyResolver for EnvProxyResolver {
    fn for_url(&self, url: &Url) -> Option<Url> {
        todo!()
    }
}
