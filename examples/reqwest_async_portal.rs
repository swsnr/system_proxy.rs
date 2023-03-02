// Copyright (c) Sebastian Wiesner <sebastian@swsnr.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! This example demonstrates how to use environment proxies and the async
//! portal resolver with the reqwest library.

#[cfg(feature = "portal")]
async fn do_request() -> Result<(), Box<dyn std::error::Error>> {
    let portal_resolver = system_proxy::unix::FreedesktopPortalProxyResolver::connect().await?;
    let env_proxies = system_proxy::env::from_curl_env();
    let proxy = reqwest::Proxy::custom(move |url| {
        let proxy = env_proxies.lookup(url).map(Clone::clone);
        println!("Environment provided proxy {proxy:?}");
        proxy.or_else(|| {
            // Create a one-shot channel to bridge from the async proxy resolver to the synchronous
            // proxy interface of reqwest.
            let (tx, rx) = tokio::sync::oneshot::channel();
            let url = url.clone();
            let portal_resolver = portal_resolver.clone();
            tokio::task::spawn(async move {
                let result = portal_resolver.lookup(&url).await;
                tx.send(result).unwrap();
            });
            let proxy = tokio::task::block_in_place(|| rx.blocking_recv())
                .unwrap()
                .unwrap_or_else(|err| {
                    eprintln!("Proxy lookup on portal failed: {}", err);
                    None
                });
            println!("Portal provided proxy {proxy:?}");
            proxy
        })
    });

    let client = reqwest::Client::builder()
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION")
        ))
        .proxy(proxy)
        .build()?;

    let response = client.get("https://httpbin.org/status/200").send().await?;
    println!("Status code: {}", response.status());
    Ok(())
}

#[cfg(feature = "portal")]
fn main() {
    // We must use a multi-threaded runtime for tokio::task::block_in_place and channels.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(do_request()).unwrap();
}

#[cfg(not(feature = "portal"))]
fn main() {
    panic!("--features portal required for this example");
}
