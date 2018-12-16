//! Fast lexical string-to-integer conversion routines.
//!
//! These routines are wrapping, and therefore can accept any buffer for any
//! size type, but will wrap to the desired value if overflow occurs.
//!
//! The following benchmarks were run on an "Intel(R) Core(TM) i7-6560U
//! CPU @ 2.20GHz" CPU, on Fedora 28, Linux kernel version 4.18.16-200
//! (x86-64), using the lexical formatter or `x.parse()`,
//! avoiding any inefficiencies in Rust string parsing. The code was
//! compiled with LTO and at an optimization level of 3.
//!
//! The benchmarks with `std` were compiled using "rustc 1.29.2 (17a9dc751
//! 2018-10-05", and the `no_std` benchmarks were compiled using "rustc
//! 1.31.0-nightly (46880f41b 2018-10-15)".
//!
//! The benchmark code may be found `benches/atoi.rs`.
//!
//! # Benchmarks
//!
//! | Type  |  lexical (ns/iter) | parse (ns/iter)       | Relative Increase |
//! |:-----:|:------------------:|:---------------------:|:-----------------:|
//! | u8    | 62,790             | 67,926                | 1.08x             |
//! | u16   | 58,896             | 76,602                | 1.30x             |
//! | u32   | 103,962            | 139,434               | 1.34x             |
//! | u64   | 192,792            | 265,931               | 1.38x             |
//! | i8    | 89,828             | 109,099               | 1.21x             |
//! | i16   | 111,592            | 140,172               | 1.26x             |
//! | i32   | 155,172            | 189,377               | 1.22x             |
//! | i64   | 197,747            | 283,541               | 1.43x             |
//!
//! # Raw Benchmarks
//!
//! ```text
//! test i8_lexical  ... bench:      89,828 ns/iter (+/- 9,172)
//! test i8_parse    ... bench:     109,099 ns/iter (+/- 2,711)
//! test i16_lexical ... bench:     111,592 ns/iter (+/- 3,862)
//! test i16_parse   ... bench:     140,172 ns/iter (+/- 7,194)
//! test i32_lexical ... bench:     155,172 ns/iter (+/- 5,248)
//! test i32_parse   ... bench:     189,377 ns/iter (+/- 10,131)
//! test i64_lexical ... bench:     197,747 ns/iter (+/- 18,041)
//! test i64_parse   ... bench:     283,541 ns/iter (+/- 14,240)
//! test u8_lexical  ... bench:      62,790 ns/iter (+/- 3,146)
//! test u8_parse    ... bench:      67,926 ns/iter (+/- 3,767)
//! test u16_lexical ... bench:      58,896 ns/iter (+/- 3,238)
//! test u16_parse   ... bench:      76,602 ns/iter (+/- 3,771)
//! test u32_lexical ... bench:     103,962 ns/iter (+/- 4,870)
//! test u32_parse   ... bench:     139,434 ns/iter (+/- 3,944)
//! test u64_lexical ... bench:     192,792 ns/iter (+/- 9,147)
//! test u64_parse   ... bench:     265,931 ns/iter (+/- 8,308)
//! ```
//!
//! Raw Benchmarks (`no_std`)
//!
//! ```text
//! test i8_lexical  ... bench:      94,142 ns/iter (+/- 5,252)
//! test i8_parse    ... bench:     107,092 ns/iter (+/- 4,121)
//! test i16_lexical ... bench:     113,284 ns/iter (+/- 17,479)
//! test i16_parse   ... bench:     141,393 ns/iter (+/- 5,804)
//! test i32_lexical ... bench:     155,704 ns/iter (+/- 5,590)
//! test i32_parse   ... bench:     191,977 ns/iter (+/- 8,241)
//! test i64_lexical ... bench:     197,485 ns/iter (+/- 11,415)
//! test i64_parse   ... bench:     298,771 ns/iter (+/- 13,941)
//! test u8_lexical  ... bench:      61,893 ns/iter (+/- 1,171)
//! test u8_parse    ... bench:      73,681 ns/iter (+/- 7,508)
//! test u16_lexical ... bench:      60,014 ns/iter (+/- 2,605)
//! test u16_parse   ... bench:      78,667 ns/iter (+/- 2,899)
//! test u32_lexical ... bench:     102,840 ns/iter (+/- 2,770)
//! test u32_parse   ... bench:     140,070 ns/iter (+/- 3,443)
//! test u64_lexical ... bench:     191,493 ns/iter (+/- 2,648)
//! test u64_parse   ... bench:     279,269 ns/iter (+/- 12,914)
//! ```

// Code the generate the benchmark plot:
//  import numpy as np
//  import pandas as pd
//  import matplotlib.pyplot as plt
//  plt.style.use('ggplot')
//  lexical = np.array([62790, 58896, 103962, 192792, 89828, 111592, 155172, 197747]) / 1e6
//  rustcore = np.array([67926, 76602, 139434, 265931, 109099, 140172, 189377, 283541]) / 1e6
//  index = ["u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64"]
//  df = pd.DataFrame({'lexical': lexical, 'rustcore': rustcore}, index = index, columns=['lexical', 'parse'])
//  ax = df.plot.bar(rot=0, figsize=(16, 8), fontsize=14, color=['#E24A33', '#348ABD'])
//  ax.set_ylabel("ms/iter")
//  ax.figure.tight_layout()
//  ax.legend(loc=2, prop={'size': 14})
//  plt.show()

use lib::{mem, ptr};
use util::*;

// ALGORITHM

/// Store a parsed digit in a variable and return the parsed digit.
/// Similar to `operator=(const T&) -> T&` in C++.
#[inline(always)]
pub(crate) unsafe fn parse_digit<T: Integer>(digit: &mut T, c: u8)
    -> T
{
    let x = char_to_digit(c);
    *digit = as_cast(x);
    *digit
}

/// Explicitly unsafe implied version of `unchecked`.
///
/// Don't trim leading zeros, since the value may be non-zero and
/// therefore invalid.
#[inline]
unsafe fn unchecked_unsafe<T>(value: &mut T, state: &mut ParseState, radix: T, last: *const u8)
    where T: Integer
{
    // Continue while we have digits.
    // Don't check for overflow, we want to avoid as many conditions
    // as possible, it leads to significant speed increases on x86-64.
    // Just note it happens, and continue on.
    // Don't add a short-circuit either, since it adds significant time
    // and we want to continue parsing until everything is done, since
    // otherwise it may give us invalid results elsewhere.
    let mut digit: T = mem::uninitialized();
    while state.curr < last && parse_digit(&mut digit, *state.curr) < radix {
        // Multiply by radix, and then add the parsed digit.
        // Assign the value regardless of whether overflow happens,
        // and merely set the overflow bool.
        let (v, o1) = value.overflowing_mul(radix);
        let (v, o2) = v.overflowing_add(digit);
        *value = v;
        if state.trunc.is_null() && (o1 | o2) {
            state.trunc = state.curr;
        }
        // Always increment the pointer.
        state.increment();
    }
}

/// Optimized, unchecked atoi implementation that uses a translation table.
///
/// Returns a pointer to the end of the parsed digits and the number of
/// digits truncated from the output (0 if no overflow).
///
/// Detects overflow, but ignores it until the end of the string. Generally
/// faster than checking and modifying logic as a result.
///
/// This is an unsafe function, just needs to be safe to use FnOnce.
#[inline]
pub(crate) fn unchecked<T>(value: &mut T, state: &mut ParseState, radix: u32, last: *const u8)
    where T: Integer
{
    unsafe { unchecked_unsafe::<T>(value, state, as_cast(radix), last) }
}

/// Explicitly unsafe implied version of `checked`.
///
/// Don't trim leading zeros, since the value may be non-zero and
/// therefore invalid.
#[inline]
unsafe fn checked_unsafe<T>(value: &mut T, state: &mut ParseState, radix: T, last: *const u8)
    where T: Integer
{
    // Continue while we have digits.
    // Don't check for overflow, we want to avoid as many conditions
    // as possible, it leads to significant speed increases on x86-64.
    // Just note it happens, and continue on.
    // Don't add a short-circuit either, since it adds significant time
    // and we want to continue parsing until everything is done, since
    // otherwise it may give us invalid results elsewhere.
    let mut digit: T = mem::uninitialized();
    while state.curr < last && parse_digit(&mut digit, *state.curr) < radix {
        // Increment our pointer, to continue parsing digits.
        // Only multiply to the radix and add the parsed digit if
        // the value hasn't overflowed yet, and only assign to the
        // original value if the operations don't overflow.
        if state.trunc.is_null() {
            // Chain these two operations before we assign, since
            // otherwise we get an invalid result.
            match value.checked_mul(radix).and_then(|v| v.checked_add(digit)) {
                // No overflow, assign the value.
                Some(v) => *value = v,
                // Overflow occurred, set truncated position
                None    => state.trunc = state.curr,
            }
        }
        // Always increment the pointer.
        state.increment();
    }
}

/// Optimized, checked atoi implementation that uses a translation table.
///
/// Returns a pointer to the end of the parsed digits and the number of
/// digits truncated from the output.
///
/// Detects overflow and aborts parsing, but increments the pointer until
/// invalid characters are found. General slower than the unchecked variant.
#[inline]
#[allow(dead_code)]
pub(crate) fn checked<T>(value: &mut T, state: &mut ParseState, radix: u32, last: *const u8)
    where T: Integer
{
    unsafe { checked_unsafe::<T>(value, state, as_cast(radix), last) }
}

/// Parse value from a positive numeric string.
#[inline]
pub(crate) unsafe fn value<T, Cb>(state: &mut ParseState, radix: u32, last: *const u8, cb: Cb)
    -> T
    where T: Integer,
          Cb: FnOnce(&mut T, &mut ParseState, u32, *const u8)
{
    debug_assert_radix!(radix);

    // Trim the leading 0s here, where we can guarantee the value is 0,
    // and therefore trimming these leading 0s is actually valid.
    state.ltrim_char(last, b'0');

    // Initialize a 0 version of our value, and invoke the low-level callback.
    let mut value: T = T::ZERO;
    cb(&mut value, state, radix, last);
    value
}

/// Handle +/- numbers and forward to implementation.
///
/// `first` must be less than or equal to `last`.
#[inline]
pub(crate) unsafe fn filter_sign<T, Cb>(state: &mut ParseState, radix: u32, last: *const u8, cb: Cb)
    -> (T, i32)
    where T: Integer,
          Cb: FnOnce(&mut T, &mut ParseState, u32, *const u8)
{
    if state.curr == last {
        (T::ZERO, 1)
    } else {
        match *state.curr {
            b'+' => {
                state.increment();
                let value = value::<T, Cb>(state, radix, last, cb);
                (value, 1)
            },
            b'-' => {
                state.increment();
                let value = value::<T, Cb>(state, radix, last, cb);
                (value, -1)
            },
            _    => {
                let value = value::<T, Cb>(state, radix, last, cb);
                (value, 1)
            },
        }
    }
}

/// Handle unsigned +/- numbers and forward to implied implementation.
//  Can just use local namespace
#[inline]
pub(crate) unsafe fn unsigned<T, Cb>(radix: u32, first: *const u8, last: *const u8, cb: Cb)
    -> (T, *const u8, bool)
    where T: UnsignedInteger,
          Cb: FnOnce(&mut T, &mut ParseState, u32, *const u8)
{
    if first == last {
        (T::ZERO, ptr::null(), false)
    } else {
        let mut state = ParseState::new(first);
        let (value, sign) = filter_sign::<T, Cb>(&mut state, radix, last, cb);
        match sign {
            -1 => (value.wrapping_neg(), state.curr, true),
            1  => (value, state.curr, state.is_truncated()),
            _  => unreachable!(),
        }
    }
}

/// Handle signed +/- numbers and forward to implied implementation.
//  Can just use local namespace
#[inline]
pub(crate) unsafe fn signed<T, Cb>(radix: u32, first: *const u8, last: *const u8, cb: Cb)
    -> (T, *const u8, bool)
    where T: SignedInteger,
          Cb: FnOnce(&mut T, &mut ParseState, u32, *const u8)
{
    if first == last {
        (T::ZERO, ptr::null(), false)
    } else {
        let mut state = ParseState::new(first);
        let (value, sign) = filter_sign::<T, Cb>(&mut state, radix, last, cb);
        match sign {
            // -value overflowing can only occur when overflow happens,
            // and specifically, when the overflow produces a value
            // of exactly T::min_value().
            -1 => (value.wrapping_neg(), state.curr, state.is_truncated()),
            1  => (value, state.curr, state.is_truncated()),
            _  => unreachable!(),
        }
    }
}

// UNSAFE API

/// Generate the unsigned, unsafe wrappers.
macro_rules! generate_unsafe_unsigned {
    ($func:ident, $t:tt) => (
        /// Unsafe, C-like importer for unsigned numbers.
        #[inline]
        unsafe fn $func(radix: u8, first: *const u8, last: *const u8)
            -> ($t, *const u8, bool)
        {
            unsigned::<$t, _>(radix.into(), first, last, unchecked::<$t>)
        }
    )
}

generate_unsafe_unsigned!(atou8_unsafe, u8);
generate_unsafe_unsigned!(atou16_unsafe, u16);
generate_unsafe_unsigned!(atou32_unsafe, u32);
generate_unsafe_unsigned!(atou64_unsafe, u64);
generate_unsafe_unsigned!(atou128_unsafe, u128);
generate_unsafe_unsigned!(atousize_unsafe, usize);

/// Generate the signed, unsafe wrappers.
macro_rules! generate_unsafe_signed {
    ($func:ident, $t:tt) => (
        /// Unsafe, C-like importer for signed numbers.
        #[inline]
        unsafe fn $func(radix: u8, first: *const u8, last: *const u8)
            -> ($t, *const u8, bool)
        {
            signed::<$t, _>(radix.into(), first, last, unchecked::<$t>)
        }
    )
}

generate_unsafe_signed!(atoi8_unsafe, i8);
generate_unsafe_signed!(atoi16_unsafe, i16);
generate_unsafe_signed!(atoi32_unsafe, i32);
generate_unsafe_signed!(atoi64_unsafe, i64);
generate_unsafe_signed!(atoi128_unsafe, i128);
generate_unsafe_signed!(atoisize_unsafe, isize);

// WRAP UNSAFE LOCAL
generate_from_bytes_local!(atou8_local, u8, atou8_unsafe);
generate_from_bytes_local!(atou16_local, u16, atou16_unsafe);
generate_from_bytes_local!(atou32_local, u32, atou32_unsafe);
generate_from_bytes_local!(atou64_local, u64, atou64_unsafe);
generate_from_bytes_local!(atou128_local, u128, atou128_unsafe);
generate_from_bytes_local!(atousize_local, usize, atousize_unsafe);
generate_from_bytes_local!(atoi8_local, i8, atoi8_unsafe);
generate_from_bytes_local!(atoi16_local, i16, atoi16_unsafe);
generate_from_bytes_local!(atoi32_local, i32, atoi32_unsafe);
generate_from_bytes_local!(atoi64_local, i64, atoi64_unsafe);
generate_from_bytes_local!(atoi128_local, i128, atoi128_unsafe);
generate_from_bytes_local!(atoisize_local, isize, atoisize_unsafe);

// RANGE API (FFI)
generate_from_range_api!(atou8_range, u8, atou8_local);
generate_from_range_api!(atou16_range, u16, atou16_local);
generate_from_range_api!(atou32_range, u32, atou32_local);
generate_from_range_api!(atou64_range, u64, atou64_local);
generate_from_range_api!(atou128_range, u128, atou128_local);
generate_from_range_api!(atousize_range, usize, atousize_local);
generate_from_range_api!(atoi8_range, i8, atoi8_local);
generate_from_range_api!(atoi16_range, i16, atoi16_local);
generate_from_range_api!(atoi32_range, i32, atoi32_local);
generate_from_range_api!(atoi64_range, i64, atoi64_local);
generate_from_range_api!(atoi128_range, i128, atoi128_local);
generate_from_range_api!(atoisize_range, isize, atoisize_local);
generate_try_from_range_api!(try_atou8_range, u8, atou8_local);
generate_try_from_range_api!(try_atou16_range, u16, atou16_local);
generate_try_from_range_api!(try_atou32_range, u32, atou32_local);
generate_try_from_range_api!(try_atou64_range, u64, atou64_local);
generate_try_from_range_api!(try_atou128_range, u128, atou128_local);
generate_try_from_range_api!(try_atousize_range, usize, atousize_local);
generate_try_from_range_api!(try_atoi8_range, i8, atoi8_local);
generate_try_from_range_api!(try_atoi16_range, i16, atoi16_local);
generate_try_from_range_api!(try_atoi32_range, i32, atoi32_local);
generate_try_from_range_api!(try_atoi64_range, i64, atoi64_local);
generate_try_from_range_api!(try_atoi128_range, i128, atoi128_local);
generate_try_from_range_api!(try_atoisize_range, isize, atoisize_local);

// SLICE API
generate_from_slice_api!(atou8_slice, u8, atou8_local);
generate_from_slice_api!(atou16_slice, u16, atou16_local);
generate_from_slice_api!(atou32_slice, u32, atou32_local);
generate_from_slice_api!(atou64_slice, u64, atou64_local);
generate_from_slice_api!(atou128_slice, u128, atou128_local);
generate_from_slice_api!(atousize_slice, usize, atousize_local);
generate_from_slice_api!(atoi8_slice, i8, atoi8_local);
generate_from_slice_api!(atoi16_slice, i16, atoi16_local);
generate_from_slice_api!(atoi32_slice, i32, atoi32_local);
generate_from_slice_api!(atoi64_slice, i64, atoi64_local);
generate_from_slice_api!(atoi128_slice, i128, atoi128_local);
generate_from_slice_api!(atoisize_slice, isize, atoisize_local);
generate_try_from_slice_api!(try_atou8_slice, u8, atou8_local);
generate_try_from_slice_api!(try_atou16_slice, u16, atou16_local);
generate_try_from_slice_api!(try_atou32_slice, u32, atou32_local);
generate_try_from_slice_api!(try_atou64_slice, u64, atou64_local);
generate_try_from_slice_api!(try_atou128_slice, u128, atou128_local);
generate_try_from_slice_api!(try_atousize_slice, usize, atousize_local);
generate_try_from_slice_api!(try_atoi8_slice, i8, atoi8_local);
generate_try_from_slice_api!(try_atoi16_slice, i16, atoi16_local);
generate_try_from_slice_api!(try_atoi32_slice, i32, atoi32_local);
generate_try_from_slice_api!(try_atoi64_slice, i64, atoi64_local);
generate_try_from_slice_api!(try_atoi128_slice, i128, atoi128_local);
generate_try_from_slice_api!(try_atoisize_slice, isize, atoisize_local);

// TESTS
// -----

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "radix")]
    const DATA: [(u8, &'static str); 35] = [
        (2, "100101"),
        (3, "1101"),
        (4, "211"),
        (5, "122"),
        (6, "101"),
        (7, "52"),
        (8, "45"),
        (9, "41"),
        (10, "37"),
        (11, "34"),
        (12, "31"),
        (13, "2B"),
        (14, "29"),
        (15, "27"),
        (16, "25"),
        (17, "23"),
        (18, "21"),
        (19, "1I"),
        (20, "1H"),
        (21, "1G"),
        (22, "1F"),
        (23, "1E"),
        (24, "1D"),
        (25, "1C"),
        (26, "1B"),
        (27, "1A"),
        (28, "19"),
        (29, "18"),
        (30, "17"),
        (31, "16"),
        (32, "15"),
        (33, "14"),
        (34, "13"),
        (35, "12"),
        (36, "11"),
    ];

    #[test]
    fn checked_test() {
        let s = "1234567891234567890123";
        unsafe {
            let first = s.as_ptr();
            let last = first.add(s.len());
            let mut value: u64 = 0;
            let mut state = ParseState::new(first);
            checked(&mut value, &mut state, 10, last);
            // check it doesn't overflow
            assert_eq!(value, 12345678912345678901);
            assert_eq!(state.curr, last);
            assert_eq!(state.truncated_bytes(), 2);
        }
    }

    #[test]
    fn unchecked_test() {
        let s = "1234567891234567890123";
        unsafe {
            let first = s.as_ptr();
            let last = first.add(s.len());
            let mut value: u64 = 0;
            let mut state = ParseState::new(first);
            unchecked(&mut value, &mut state, 10, last);
            // check it does overflow
            assert_eq!(value, 17082782369737483467);
            assert_eq!(state.curr, last);
            assert_eq!(state.truncated_bytes(), 2);
        }
    }

    #[test]
    fn atou8_base10_test() {
        assert_eq!(0, atou8_slice(10, b"0"));
        assert_eq!(127, atou8_slice(10, b"127"));
        assert_eq!(128, atou8_slice(10, b"128"));
        assert_eq!(255, atou8_slice(10, b"255"));
        assert_eq!(255, atou8_slice(10, b"-1"));
        assert_eq!(1, atou8_slice(10, b"1a"));
    }

    #[cfg(feature = "radix")]
    #[test]
    fn atou8_basen_test() {
        for (b, s) in DATA.iter() {
            assert_eq!(atou8_slice(*b, s.as_bytes()), 37);
        }
    }

    #[test]
    fn atoi8_base10_test() {
        assert_eq!(0, atoi8_slice(10, b"0"));
        assert_eq!(127, atoi8_slice(10, b"127"));
        assert_eq!(-128, atoi8_slice(10, b"128"));
        assert_eq!(-1, atoi8_slice(10, b"255"));
        assert_eq!(-1, atoi8_slice(10, b"-1"));
        assert_eq!(1, atoi8_slice(10, b"1a"));
    }

    #[test]
    fn atou16_base10_test() {
        assert_eq!(0, atou16_slice(10, b"0"));
        assert_eq!(32767, atou16_slice(10, b"32767"));
        assert_eq!(32768, atou16_slice(10, b"32768"));
        assert_eq!(65535, atou16_slice(10, b"65535"));
        assert_eq!(65535, atou16_slice(10, b"-1"));
        assert_eq!(1, atou16_slice(10, b"1a"));
    }

    #[test]
    fn atoi16_base10_test() {
        assert_eq!(0, atoi16_slice(10, b"0"));
        assert_eq!(32767, atoi16_slice(10, b"32767"));
        assert_eq!(-32768, atoi16_slice(10, b"32768"));
        assert_eq!(-1, atoi16_slice(10, b"65535"));
        assert_eq!(-1, atoi16_slice(10, b"-1"));
        assert_eq!(1, atoi16_slice(10, b"1a"));
    }

    #[cfg(feature = "radix")]
    #[test]
    fn atoi16_basen_test() {
        assert_eq!(atoi16_slice(36, b"YA"), 1234);
    }

    #[test]
    fn atou32_base10_test() {
        assert_eq!(0, atou32_slice(10, b"0"));
        assert_eq!(2147483647, atou32_slice(10, b"2147483647"));
        assert_eq!(2147483648, atou32_slice(10, b"2147483648"));
        assert_eq!(4294967295, atou32_slice(10, b"4294967295"));
        assert_eq!(4294967295, atou32_slice(10, b"-1"));
        assert_eq!(1, atou32_slice(10, b"1a"));
    }

    #[test]
    fn atoi32_base10_test() {
        assert_eq!(0, atoi32_slice(10, b"0"));
        assert_eq!(2147483647, atoi32_slice(10, b"2147483647"));
        assert_eq!(-2147483648, atoi32_slice(10, b"2147483648"));
        assert_eq!(-1, atoi32_slice(10, b"4294967295"));
        assert_eq!(-1, atoi32_slice(10, b"-1"));
        assert_eq!(1, atoi32_slice(10, b"1a"));
    }

    #[test]
    fn atou64_base10_test() {
        assert_eq!(0, atou64_slice(10, b"0"));
        assert_eq!(9223372036854775807, atou64_slice(10, b"9223372036854775807"));
        assert_eq!(9223372036854775808, atou64_slice(10, b"9223372036854775808"));
        assert_eq!(18446744073709551615, atou64_slice(10, b"18446744073709551615"));
        assert_eq!(18446744073709551615, atou64_slice(10, b"-1"));
        assert_eq!(1, atou64_slice(10, b"1a"));
    }

    #[test]
    fn atoi64_base10_test() {
        assert_eq!(0, atoi64_slice(10, b"0"));
        assert_eq!(9223372036854775807, atoi64_slice(10, b"9223372036854775807"));
        assert_eq!(-9223372036854775808, atoi64_slice(10, b"9223372036854775808"));
        assert_eq!(-1, atoi64_slice(10, b"18446744073709551615"));
        assert_eq!(-1, atoi64_slice(10, b"-1"));
        assert_eq!(1, atoi64_slice(10, b"1a"));

        // Add tests discovered via fuzzing.
        assert_eq!(2090691195633139712, atoi64_slice(10, b"406260572150672006000066000000060060007667760000000000000000000+00000006766767766666767665670000000000000000000000666"));
    }
    #[test]
    fn try_atou8_base10_test() {
        assert_eq!(invalid_digit_error(0, 0), try_atou8_slice(10, b""));
        assert_eq!(success(0), try_atou8_slice(10, b"0"));
        assert_eq!(invalid_digit_error(1, 1), try_atou8_slice(10, b"1a"));
        assert_eq!(overflow_error(0), try_atou8_slice(10, b"256"));
    }

    #[test]
    fn try_atoi8_base10_test() {
        assert_eq!(invalid_digit_error(0, 0), try_atoi8_slice(10, b""));
        assert_eq!(success(0), try_atoi8_slice(10, b"0"));
        assert_eq!(invalid_digit_error(1, 1), try_atoi8_slice(10, b"1a"));
        assert_eq!(overflow_error(-128), try_atoi8_slice(10, b"128"));
    }

    #[test]
    fn try_atou16_base10_test() {
        assert_eq!(invalid_digit_error(0, 0), try_atou16_slice(10, b""));
        assert_eq!(success(0), try_atou16_slice(10, b"0"));
        assert_eq!(invalid_digit_error(1, 1), try_atou16_slice(10, b"1a"));
        assert_eq!(overflow_error(0), try_atou16_slice(10, b"65536"));
    }

    #[test]
    fn try_atoi16_base10_test() {
        assert_eq!(invalid_digit_error(0, 0), try_atoi16_slice(10, b""));
        assert_eq!(success(0), try_atoi16_slice(10, b"0"));
        assert_eq!(invalid_digit_error(1, 1), try_atoi16_slice(10, b"1a"));
        assert_eq!(overflow_error(-32768), try_atoi16_slice(10, b"32768"));
    }

    #[test]
    fn try_atou32_base10_test() {
        assert_eq!(invalid_digit_error(0, 0), try_atou32_slice(10, b""));
        assert_eq!(success(0), try_atou32_slice(10, b"0"));
        assert_eq!(invalid_digit_error(1, 1), try_atou32_slice(10, b"1a"));
        assert_eq!(overflow_error(0), try_atou32_slice(10, b"4294967296"));
    }

    #[test]
    fn try_atoi32_base10_test() {
        assert_eq!(invalid_digit_error(0, 0), try_atoi32_slice(10, b""));
        assert_eq!(success(0), try_atoi32_slice(10, b"0"));
        assert_eq!(invalid_digit_error(1, 1), try_atoi32_slice(10, b"1a"));
        assert_eq!(overflow_error(-2147483648), try_atoi32_slice(10, b"2147483648"));
    }

    #[test]
    fn try_atou64_base10_test() {
        assert_eq!(invalid_digit_error(0, 0), try_atou64_slice(10, b""));
        assert_eq!(success(0), try_atou64_slice(10, b"0"));
        assert_eq!(invalid_digit_error(1, 1), try_atou64_slice(10, b"1a"));
        assert_eq!(overflow_error(0), try_atou64_slice(10, b"18446744073709551616"));
    }

    #[test]
    fn try_atoi64_base10_test() {
        assert_eq!(invalid_digit_error(0, 0), try_atoi64_slice(10, b""));
        assert_eq!(success(0), try_atoi64_slice(10, b"0"));
        assert_eq!(invalid_digit_error(1, 1), try_atoi64_slice(10, b"1a"));
        assert_eq!(overflow_error(-9223372036854775808), try_atoi64_slice(10, b"9223372036854775808"));

        // Add tests discovered via fuzzing.
        assert_eq!(overflow_error(-9223372036854775808), try_atoi64_slice(10, b"-000000000000000000000000066000000000000000000000000000000000000000000695092744062605721500000000695092744062600000000000000000000000000000000000000000000000000000000000000?0000000000000000000000000000000000000000000000000\x100000000006666600000000006000000066666666000766776676677000676766509274406260572150000000069509274406260572150000000000000000000000000000000000066000000000000000000000000000000000000000000600000950927440626057215000000006950927440062600057215000000666600666666666600001000000676766766766770000666000766776676000000000000000000000000006950927440626666676676676676660066666000000000060000000600000000000000000000000000000000000+?676677000695092744"));
        assert_eq!(overflow_error(2090691195633139712), try_atoi64_slice(10, b"406260572150672006000066000000060060007667760000000000000000000+00000006766767766666767665670000000000000000000000666"));
        assert_eq!(overflow_error(7125759012462002176), try_atoi64_slice(10, b"6260572000000000000000-3*+\x006666600099000066006660066665?666666666599990000666"));
    }
}
