// Copyright (c) 2022 Sebastian Wiesner <sebastian@swsnr.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.[cfg(test)]

//! Provide [`WinHttpProxyResolver`] which resolves the HTTP proxies through the WinHttp API, which
//! uses the system-wide windows proxy settings, and supports PAC urls, and even automatic
//! discovery of PAC URLs.

use std::ffi::c_void;

use url::Url;
use windows::core::*;
use windows::Win32::Networking::WinHttp::*;

#[derive(Debug)]
pub struct WinHttpProxyResolver {
    hsession: *mut c_void,
}

impl WinHttpProxyResolver {
    /// Create a proxy resolver for the given WinHttp session.
    ///
    /// `hsession` is a WinHttp session handle to use; the proxy resolver **takes ownership** of
    /// the handle, and will call `WinHttpCloseHandle` on `hsession` when dropped.
    ///
    /// Return a resolver using the WinHttp session referred to by `hsession` or `None` if
    /// `hsession` was `NULL`.
    ///
    /// ## Safety
    ///
    /// This function does not verify the type of `hsession`; it's the caller's responsibility to
    /// ensure that `hsession` is a valid WinHttp session handle, as returned by `WinHttpOpen`;
    pub unsafe fn for_session(hsession: *mut c_void) -> Option<Self> {
        if hsession.is_null() {
            None
        } else {
            Some(Self { hsession })
        }
    }

    /// Create a new proxy resolver on a fresh WinHttp session.
    ///
    /// Return the resolver or an error if creating the session handle failed.
    pub fn new_session() -> std::io::Result<Self> {
        unsafe {
            // We don't really care for any of the following parameters as we'll never use this
            // handle to make HTTP calls, but let's follow good practices anyway.
            let hsession = WinHttpOpen(
                "system_proxy,rs",
                WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
                PCWSTR(std::ptr::null()), /* WINHTTP_NO_PROXY_NAME */
                PCWSTR(std::ptr::null()), /* WINHTTP_NO_PROXY_BYPASS */
                WINHTTP_FLAG_SECURE_DEFAULTS,
            );
            Self::for_session(hsession)
        }
        .ok_or_else(|| std::io::Error::last_os_error())
    }

    pub fn get_proxy_for_url(&self, url: &Url) -> std::io::Result<String> {
        let mut proxy_info = WINHTTP_PROXY_INFO::default();
        let mut autoproxy_options = WINHTTP_AUTOPROXY_OPTIONS::default();
        autoproxy_options.dwFlags = WINHTTP_AUTOPROXY_ALLOW_AUTOCONFIG
            | WINHTTP_AUTOPROXY_ALLOW_CM
            | WINHTTP_AUTOPROXY_ALLOW_STATIC
            | WINHTTP_AUTOPROXY_AUTO_DETECT;
        autoproxy_options.dwAutoDetectFlags =
            WINHTTP_AUTO_DETECT_TYPE_DHCP | WINHTTP_AUTO_DETECT_TYPE_DNS_A;
        autoproxy_options.fAutoLogonIfChallenged = true.into();
        unsafe {
            let successful = !WinHttpGetProxyForUrl(
                self.hsession,
                url.as_str(),
                &mut autoproxy_options,
                &mut proxy_info,
            )
            .as_bool();
            if !successful {
                Err(std::io::Error::last_os_error())
            } else {
                if proxy_info.dwAccessType == WINHTTP_ACCESS_TYPE_NO_PROXY {
                    free(proxy_info);
                    None
                } else {
                    let proxy_list: String = proxy_info.lpszProxy.into();
                    free(proxy_info);
                    proxy_list.split(&[' ', ''])
                    todo!()
                }
            }
        }
    }
}

fn free(proxy_info: WINHTTP_PROXY_INFO) {
    todo!("Properly free strings");
    if !proxy_info.lpszProxy.is_null() {
        // GlobalFree(proxy_info.lpszProxy);
    }
    if !proxy_info.lpszProxyBypass.is_null() {
        // GlobalFree(proxy_info.lpszProxyBypass);
    }
    if !proxy_info.lpszProxyBypass.is_null() {
        GlobalFree(proxy_info.lpszProxy);
    }
}

impl Drop for WinHttpProxyResolver {
    fn drop(&mut self) {
        if self.hsession.is_null() {
            log::warn!("WinHttpProxyResolver held a NULL session handle unexpectedly")
        } else {
            let is_closed = unsafe { WinHttpCloseHandle(self.hsession) }.as_bool();
            if !is_closed {
                let error = std::io::Error::last_os_error();
                log::error!("Failed to close WinHttp session handle: {}", error);
            }
        }
    }
}

impl Default for WinHttpProxyResolver {
    /// Create a new resolver on a new WinHttp session.
    ///
    /// **If creating a new WinHttp session fails this function panics.**  Normally that's
    /// absolutely fine as things need to go badly wrong for WinHttp to fail, but if you'd like
    /// more control over the error behaviour refer to [`WinHttpProxyResolver::new_session`].
    fn default() -> Self {
        Self::new_session().unwrap()
    }
}

impl crate::ProxyResolver for WinHttpProxyResolver {
    fn for_url(&self, url: &Url) -> Option<Url> {
        todo!()
    }
}
