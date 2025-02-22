#![feature(portable_simd)]
#![feature(const_ptr_read)]
#![feature(const_trait_impl)]
#![feature(const_deref)]
#![feature(assert_matches)]
#![feature(round_char_boundary)]

#![allow(clippy::let_unit_value)]
#![allow(clippy::unit_arg)]
// #![warn(missing_docs)]
// Copyright 2012-2022 The Rust Project Developers and Eric Kidd and Christopher Moore.  See the
// COPYRIGHT-RUST.txt file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed except
// according to those terms.

//! A simple library implementing the [CESU-8 compatibility encoding
//! scheme](http://www.unicode.org/reports/tr26/tr26-2.html).  This is a
//! non-standard variant of UTF-8 that is used internally by some systems
//! that need to represent UTF-16 data as 8-bit characters.  Yes, this is
//! ugly.
//!
//! Use of this encoding is discouraged by the Unicode Consortium.  It's OK
//! for working with existing internal APIs, but it should not be used for
//! transmitting or storing data.
//!
//! ```
//! use std::borrow::Cow;
//! use cesu8str::Cesu8Str;
//!
//! // 16-bit Unicode characters are the same in UTF-8 and CESU-8.
//! const TEST_STRING: &str = "aé日";
//! const TEST_UTF8: &[u8] = TEST_STRING.as_bytes();
//! assert_eq!(TEST_UTF8, Cesu8Str::from_utf8(TEST_STRING).as_bytes());
//! let cesu_from_bytes = Cesu8Str::try_from_bytes(TEST_UTF8).unwrap();
//! assert_eq!(TEST_UTF8, cesu_from_bytes.as_bytes());
//!
//! // This string is CESU-8 data containing a 6-byte surrogate pair,
//! // which decodes to a 4-byte UTF-8 string.
//! let data = &[0xED, 0xA0, 0x81, 0xED, 0xB0, 0x81];
//! assert_eq!("\u{10401}", Cesu8Str::try_from_bytes(data).unwrap().to_str());
//! ```
//!
//! ### A note about security
//!
//! While this library tries it's best to fail and check for malformed
//! input, this is a legacy data format that should only be used for
//! interacting with legacy libraries. CESU-8 is intended as an
//! internal-only format, malformed data should be assumed to be improperly
//! encoded (a bug), or an attacker.
//!
//! ### Java and U+0000, and other variants
//!
//! Java uses the CESU-8 encoding as described above, but with one
//! difference: The null character U+0000 is represented as an overlong
//! UTF-8 sequence `C0 80`. This is supported by the `Cesu8Str::from_cesu8(bytes, Variant::Java)` and
//! `java_variant_str.as_bytes()` methods.
//!
//! ### Surrogate pairs and UTF-8
//!
//! The UTF-16 encoding uses "surrogate pairs" to represent Unicode code
//! points in the range from U+10000 to U+10FFFF.  These are 16-bit numbers
//! in the range 0xD800 to 0xDFFF.
//!
//! * 0xD800 to 0xDBFF: First half of surrogate pair.  When encoded as
//!   CESU-8, these become **1110**1101 **10**100000 **10**000000 to
//!   **1110**1101 **10**101111 **10**111111.
//!
//! * 0xDC00 to 0xDFFF: Second half of surrogate pair.  These become
//!   **1110**1101 **10**110000 **10**000000 to
//!   **1110**1101 **10**111111 **10**111111.
//!
//! Wikipedia [explains](http://en.wikipedia.org/wiki/UTF-16) the
//! code point to UTF-16 conversion process:
//!
//! > Consider the encoding of U+10437 (𐐷):
//! >
//! > * Subtract 0x10000 from 0x10437. The result is 0x00437, 0000 0000 0100
//! >   0011 0111.
//! > * Split this into the high 10-bit value and the low 10-bit value:
//! >   0000000001 and 0000110111.
//! > * Add 0xD800 to the high value to form the high surrogate: 0xD800 +
//! >   0x0001 = 0xD801.
//! > * Add 0xDC00 to the low value to form the low surrogate: 0xDC00 +
//! >   0x0037 = 0xDC37.

#![warn(missing_docs)]

mod decoding;
mod encoding;
mod legacy_api;
mod string;
mod string_impls;
#[rustfmt::skip]
mod unicode;

/// # Next-generation strings
/// 
/// | `encode_nul` | `nul_terminated` | `mode` | `Output` |
/// | - | - | - |- |
/// | `false` | `false` | `borrowed` | `Cesu8Str` |
/// | `false` | `false` | `owned` | `Cesu8String` |
/// | `false` | `true` | `borrowed` | n/a |
/// | `false` | `true` | `owned` | n/a |
/// | `true` | `false` | `borrowed` | `Mutf8Str` |
/// | `true` | `false` | `owned` | `Mutf8String` |
/// | `true` | `true` | `borrowed` | `Mutf8CStr` |
/// | `true` | `true` | `owned` | `Mutf8CString` |
/// 
mod ngstr;

/// A prelude including most relavent structs for the crate, including re-exports of some stdlib helpers
/// such as `Cow`, `CStr`, `CString`, and `Deref`.
pub mod prelude {
    pub use crate::ngstr::prelude::*;
}

pub use ngstr::{
    NGCesu8CError,
    cesu8str::Cesu8Str,
    cesu8string::Cesu8String,
    mutf8str::Mutf8Str,
    mutf8string::Mutf8String,
    mutf8cstr::Mutf8CStr,
    mutf8cstring::Mutf8CString,
    mutf8cstr::FromStrWithNulError,
    mutf8cstring::FromMutf8BytesWithNulError,

    TryFromUtf8Error,
    FromBytesWithNulError,
};

pub use crate::decoding::Cesu8Error;
pub use crate::legacy_api::*;
#[allow(deprecated)]
pub use crate::string::Cesu8Str as LegacyCesu8Str;
pub(crate) use crate::string::default_cesu8_capacity;

#[cfg(test)]
mod tests;

/// Which variant of the encoding are we working with?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// Regular CESU-8, with '\0' represented as itself.
    Standard,

    /// This is technically Java's "Modified UTF-8", which is supposedly
    /// like CESU-8, except that it UTF-8 encodes the '\0' byte.  I'm sure
    /// it seemed like a good idea at the time.
    Java,
}

impl Variant {
    /// Returns true if this Variant of CESU-8 converts nul-bytes to a `&[0xC0, 0x80]` sequence.
    ///
    /// This should only be true for the Java variant of CESU-8, also known as Modified UTF-8.
    pub const fn encodes_nul(&self) -> bool {
        match self {
            Variant::Standard => false,
            Variant::Java => true,
        }
    }
}

// Currently using a const generic bool for specializing functions for each variant
// once const_generics is stabalized, we can use the variant directly
// Creations of Cesu8Str should use this impl, then this impl can be removed once
// const_generics is stabalized and we can adjust things properly
#[doc(hidden)]
impl From<bool> for Variant {
    fn from(b: bool) -> Variant {
        match b {
            false => Variant::Standard,
            true => Variant::Java,
        }
    }
}

#[inline]
#[track_caller]
pub(crate) fn from_utf8_slice<'s>(by: &'s [u8], expect_msg: &'_ str) -> &'s str {
    if cfg!(debug_assertions) || cfg!(validate_release) {
        std::str::from_utf8(by).expect(expect_msg)
    } else {
        unsafe { std::str::from_utf8_unchecked(by) }
    }
}

#[inline]
#[track_caller]
pub(crate) fn from_utf8_vec(by: Vec<u8>, expect_msg: &str) -> String {
    if cfg!(debug_assertions) || cfg!(validate_release) {
        String::from_utf8(by).expect(expect_msg)
    } else {
        unsafe { String::from_utf8_unchecked(by) }
    }
}
