// Copyright (c) 2022 Sebastian Wiesner <sebastian@swsnr.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

fn main() {
    let proxy = system_proxy::default();
    let client = reqwest::blocking::Client::builder()
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION")
        ))
        .proxy(reqwest::Proxy::custom(move |url| {
            let proxy_url = proxy.for_url(url);
            match &proxy_url {
                None => println!("Using direct connection for URL {}", url),
                Some(u) => println!("Using proxy {} for URL {}", u, url),
            }
            proxy_url
        }))
        .build()
        .unwrap();

    let response = client.get("https://httpbin.org/status/200").send().unwrap();
    println!("Status code: {}", response.status());
}
