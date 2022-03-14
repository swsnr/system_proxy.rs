# system_proxy.rs

[![Current release](https://img.shields.io/crates/v/system_proxy.svg)][crates]
[![Documentation](https://docs.rs/system_proxy/badge.svg)][docs]

Resolve system HTTP(S) proxies for URLs.

[crates]: https://crates.io/crates/system_proxy
[docs]: https://docs.rs/system_proxy

```rust
let proxy = system_proxy::default();
let client = reqwest::blocking::Client::builder()
    .user_agent(concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")))
    .proxy(reqwest::Proxy::custom(move |u| proxy.for_url(u)))
    .build()
    .unwrap();

let response = client.get("https://httpbin.org/status/200").send().unwrap();
println!("Status code: {}", response.status());
```

See the [module level documentation](https://docs.rs/system_proxy/latest/system_proxy/) for more information.

## License

Copyright 2022 Sebastian Wiesner <sebastian@swsnr.de>

This Source Code is subject to the terms of the Mozilla Public License, v. 2.0.
See `LICENSE` or <https://mozilla.org/MPL/2.0/> for a copy of the license.
