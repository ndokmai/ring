// Copyright 2016 Brian Smith.
// Portions Copyright (c) 2016, Google Inc.
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

use super::block::{Block, BLOCK_LEN};
use crate::{
    c,
    polyfill::{convert::*, slice::u32_from_le_u8},
};
use core;

#[repr(C)]
pub struct Key([Block; KEY_BLOCKS]);

impl<'a> From<&'a [u8; KEY_LEN]> for Key {
    fn from(value: &[u8; KEY_LEN]) -> Self { Key(<[Block; KEY_BLOCKS]>::from_(value)) }
}

#[inline]
pub fn chacha20_xor_in_place(key: &Key, counter: &Counter, in_out: &mut [u8]) {
    unsafe {
        chacha20_xor_inner(
            key,
            counter,
            in_out.as_ptr(),
            in_out.len(),
            in_out.as_mut_ptr(),
        );
    }
}

pub fn chacha20_xor_overlapping(
    key: &Key, counter: &Counter, in_out: &mut [u8], in_prefix_len: usize,
) {
    // XXX: The x86 and at least one branch of the ARM assembly language
    // code doesn't allow overlapping input and output unless they are
    // exactly overlapping. TODO: Figure out which branch of the ARM code
    // has this limitation and come up with a better solution.
    //
    // https://rt.openssl.org/Ticket/Display.html?id=4362
    let len = in_out.len() - in_prefix_len;
    if cfg!(any(target_arch = "arm", target_arch = "x86")) && in_prefix_len != 0 {
        unsafe {
            core::ptr::copy(in_out[in_prefix_len..].as_ptr(), in_out.as_mut_ptr(), len);
        }
        chacha20_xor_in_place(key, &counter, &mut in_out[..len]);
    } else {
        unsafe {
            chacha20_xor_inner(
                key,
                counter,
                in_out[in_prefix_len..].as_ptr(),
                len,
                in_out.as_mut_ptr(),
            );
        }
    }
}

#[inline]
unsafe fn chacha20_xor_inner(
    key: &Key, counter: &Counter, input: *const u8, in_out_len: usize, output: *mut u8,
) {
    extern "C" {
        fn GFp_ChaCha20_ctr32(
            out: *mut u8, in_: *const u8, in_len: c::size_t, key: &Key, counter: &Counter,
        );
    }
    GFp_ChaCha20_ctr32(output, input, in_out_len, key, counter);
}

pub type Counter = [u32; 4];

#[inline]
pub fn make_counter(nonce: &[u8; NONCE_LEN], counter: u32) -> Counter {
    [
        counter.to_le(),
        u32_from_le_u8(nonce[0..4].try_into_().unwrap()),
        u32_from_le_u8(nonce[4..8].try_into_().unwrap()),
        u32_from_le_u8(nonce[8..12].try_into_().unwrap()),
    ]
}

const KEY_BLOCKS: usize = 2;
pub const KEY_LEN: usize = KEY_BLOCKS * BLOCK_LEN;

pub const NONCE_LEN: usize = 12; // 96 bits

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test;

    // This verifies the encryption functionality provided by ChaCha20_ctr32
    // is successful when either computed on disjoint input/output buffers,
    // or on overlapping input/output buffers. On some branches of the 32-bit
    // x86 and ARM code the in-place operation fails in some situations where
    // the input/output buffers are not exactly overlapping. Such failures are
    // dependent not only on the degree of overlapping but also the length of
    // the data. `open()` works around that by moving the input data to the
    // output location so that the buffers exactly overlap, for those targets.
    // This test exists largely as a canary for detecting if/when that type of
    // problem spreads to other platforms.
    #[test]
    pub fn chacha20_tests() {
        test::from_file("src/aead/chacha_tests.txt", |section, test_case| {
            assert_eq!(section, "");

            let key = test_case.consume_bytes("Key");
            let key: &[u8; KEY_LEN] = key.as_slice().try_into_()?;
            let key = Key::from(key);

            let ctr = test_case.consume_usize("Ctr");
            let nonce_bytes = test_case.consume_bytes("Nonce");
            let nonce: &[u8; NONCE_LEN] = nonce_bytes.as_slice().try_into_().unwrap();
            let ctr = make_counter(&nonce, ctr as u32);
            let input = test_case.consume_bytes("Input");
            let output = test_case.consume_bytes("Output");

            // Pre-allocate buffer for use in test_cases.
            let mut in_out_buf = vec![0u8; input.len() + 276];

            // Run the test case over all prefixes of the input because the
            // behavior of ChaCha20 implementation changes dependent on the
            // length of the input.
            for len in 0..(input.len() + 1) {
                chacha20_test_case_inner(
                    &key,
                    &ctr,
                    &input[..len],
                    &output[..len],
                    len,
                    &mut in_out_buf,
                );
            }

            Ok(())
        });
    }

    fn chacha20_test_case_inner(
        key: &Key, ctr: &Counter, input: &[u8], expected: &[u8], len: usize, in_out_buf: &mut [u8],
    ) {
        // Straightforward encryption into disjoint buffers is computed
        // correctly.
        unsafe {
            chacha20_xor_inner(
                key,
                &ctr,
                input[..len].as_ptr(),
                len,
                in_out_buf.as_mut_ptr(),
            )
        }
        assert_eq!(&in_out_buf[..len], expected);

        // Do not test offset buffers for x86 and ARM architectures (see above
        // for rationale).
        let max_offset = if cfg!(any(target_arch = "x86", target_arch = "arm")) {
            0
        } else {
            259
        };

        // Check that in-place encryption works successfully when the pointers
        // to the input/output buffers are (partially) overlapping.
        for alignment in 0..16 {
            for offset in 0..(max_offset + 1) {
                in_out_buf[alignment + offset..][..len].copy_from_slice(input);
                chacha20_xor_overlapping(key, ctr, &mut in_out_buf[alignment..], offset);
                assert_eq!(&in_out_buf[alignment..][..len], expected);
            }
        }
    }
}
