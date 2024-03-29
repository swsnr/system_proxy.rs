// Copyright (c) Sebastian Wiesner <sebastian@swsnr.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! This example demonstrates how to use environment proxies with
//! the reqwest library.

fn main() {
    let proxy = system_proxy::env::from_curl_env();
    let client = reqwest::blocking::Client::builder()
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION")
        ))
        .proxy(reqwest::Proxy::custom(move |url| {
            let proxy_url = proxy.lookup(url);
            match &proxy_url {
                None => println!("Using direct connection for URL {}", url),
                Some(u) => println!("Using proxy {} for URL {}", u, url),
            }
            proxy_url.cloned()
        }))
        .build()
        .unwrap();

    let response = client.get("https://httpbin.org/status/200").send().unwrap();
    println!("Status code: {}", response.status());
}
