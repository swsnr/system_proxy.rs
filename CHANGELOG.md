All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Add `FreedesktopPortalProxyResolver` to lookup proxies via flatpak portal.

### Changed
- Simplify `env` module after removal of `ProxyResolver`.

### Removed

- Drop trait `ProxyResolver`.
  Introduced prematurely; instead wait until a common API matures.
- Make Gio proxy resolver asynchronous.

## [0.2.0] – 2023-03-02

### Changed
- Update MSRV to 1.66.
- Update dependencies.

## [0.1.3] – 2022-12-01

### Changed
- Change Github URL to <https://github.com/swsnr/system_proxy.rs>.

## [0.1.2] – 2022-10-13

### Changed
- Build on Windows.
- Migrate repository to <https://github.com/lunaryorn/system_proxy.rs>.

## [0.1.1] – 2022-03-16

### Fixed
- Correctly export fallback resolver if `gnome` feature is disabled (see [#8]).

[#8]: https://codeberg.org/flausch/system_proxy.rs/issues/8

## [0.1.0] - 2022-03-14

### Added
- Support for HTTP proxies in environment variables.
- Support for Gnome system proxy.

[Unreleased]: https://github.com/swsnr/system_proxy.rs/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/swsnr/system_proxy.rs/compare/v0.1.3...v0.2.0
[0.1.3]: https://github.com/swsnr/system_proxy.rs/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/swsnr/system_proxy.rs/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/swsnr/system_proxy.rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/swsnr/system_proxy.rs/releases/tag/v0.1.0

