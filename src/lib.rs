// Copyright 2015-2016 Brian Smith.
//
// Permission to use, copy, modify, and/or distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHORS DISCLAIM ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR ANY
// SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION
// OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR IN
// CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

//! Safe, fast, small crypto using Rust with BoringSSL's cryptography
//! primitives.
//!
//! <code>git clone https://github.com/briansmith/ring</code>
//!
//! # Feature Flags
//!
//! <table>
//! <tr><th>Feature
//!     <th>Description
//! <tr><td><code>dev_urandom_fallback (default)</code>
//!     <td>This is only applicable to Linux. On Linux, by default,
//!         <code>ring::rand::SystemRandom</code> will fall back to reading
//!         from <code>/dev/urandom</code> if the <code>getrandom()</code>
//!         syscall isn't supported at runtime. When the
//!         <code>dev_urandom_fallback</code> feature is disabled, such
//!         fallbacks will not occur. See the documentation for
//!         <code>rand::SystemRandom</code> for more details.
//! <tr><td><code>force_std_detection</code>
//!     <td>This is only applicable to x86. By default, <i>ring</i> will use
//!         custom logic with the CPUID instruction to figure out which CPU
//!         features are available. With this feature, the standard
//!         <code>std::is_x86_feature_detected</code> macro will be used
//!         instead.
//! <tr><td><code>use_heap (default)</code>
//!     <td>Enable features that require use of the heap, RSA in particular.
//! </table>

#![doc(html_root_url = "https://briansmith.org/rustdoc/")]
#![feature(stdsimd)]
#![allow(
    missing_copy_implementations,
    missing_debug_implementations,
    non_camel_case_types,
    non_snake_case,
    unsafe_code
)]
// `#[derive(...)]` uses `trivial_numeric_casts` and `unused_qualifications`
// internally.
#![deny(
    missing_docs,
    trivial_numeric_casts,
    //unstable_features, // Used by `internal_benches`
    unused_qualifications,
    variant_size_differences,
)]
#![forbid(
    anonymous_parameters,
    trivial_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_results,
    warnings
)]
#![cfg_attr(
    any(
        target_os = "redox",
        all(
            not(test),
            not(feature = "use_heap"),
            unix,
            not(any(target_os = "macos", target_os = "ios")),
            any(not(target_os = "linux"), feature = "dev_urandom_fallback")
        )
    ),
    no_std
)]
#![cfg_attr(feature = "internal_benches", allow(unstable_features))]
#![cfg_attr(feature = "internal_benches", feature(test))]

#[macro_use]
mod debug;

#[macro_use]
mod bssl;

#[macro_use]
mod polyfill;

#[cfg(any(test, feature = "use_heap"))]
#[macro_use]
pub mod test;

mod arithmetic;

pub mod aead;
pub mod agreement;

mod bits;

pub mod constant_time;

pub mod io;

mod cpu;
pub mod digest;
mod ec;
mod endian;
pub mod error;
pub mod hkdf;
pub mod hmac;
mod limb;
pub mod pbkdf2;
mod pkcs8;
pub mod rand;

#[cfg(feature = "use_heap")]
mod rsa;

pub mod signature;

mod sealed {
    /// Traits that are designed to only be implemented internally in *ring*.
    //
    // Usage:
    // ```
    // use crate::sealed;
    //
    // pub trait MyType: sealed::Sealed {
    //     // [...]
    // }
    //
    // impl sealed::Sealed for MyType {}
    // ```
    pub trait Sealed {}
}

/// # Information about using *ring* in SGX
///
/// ## CPU feature detection
/// On `x86_64-fortanix-unknown-sgx`, feature detection is done using the
/// `std::is_x86_feature_detected` macro, which currently only supports
/// features enabled at compile-time. You must enable at least the `aes` and
/// `pclmul` features, otherwise *ring* will panic at runtime. See the [GitHub
/// issue](https://github.com/fortanix/rust-sgx/issues/26) for more
/// information.
///
/// To set compile-time features, you can either specify them as an environment
/// variable:
///
/// ```text
/// RUSTFLAGS="-C target-feature=+aes,+pclmul"
/// ```
///
/// Or you may configure them per target in [`.cargo/config`].
///
/// [`.cargo/config`]: https://doc.rust-lang.org/cargo/reference/config.html#configuration-keys
///
/// ## Entropy source
/// The entropy source used in SGX is the hardware random number generator
/// provided by the RDRAND instruction.
///
/// ## Nightly only
/// The `x86_64-fortanix-unknown-sgx` target is only available on nightly, and
/// *ring* Continuous Builds only build it for nightly. See the [GitHub
/// issue](https://github.com/briansmith/ring/issues/779) for more information.
///
/// ## Continuous Testing
/// While the *ring* test suite works in SGX, and it is run manually from time
/// to time, it doesn't run automatically as part of a Continuous Testing
/// setup. See the [GitHub issue](https://github.com/briansmith/ring/issues/778)
/// for more information.
#[cfg(target_env = "sgx")]
pub mod sgx {}
