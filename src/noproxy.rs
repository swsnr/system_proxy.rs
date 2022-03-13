// Copyright (c) 2022 Sebastian Wiesner <sebastian@swsnr.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.[cfg(test)]

use url::Url;

/// A proxy resolver which never returns a proxy URI.
///
/// Used as fallback if no appropriate system proxy is known.
#[derive(Default)]
pub struct NoProxy {}

impl crate::types::ProxyResolver for NoProxy {
    fn for_url(&self, _url: &Url) -> Option<Url> {
        None
    }
}
