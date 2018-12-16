//! Config settings for lexical.

// GLOBALS

/// Not a Number literal
///
/// To change the expected representation of NaN as a string,
/// change this value during before using lexical.
///
/// Do not modify this value in threaded-code, as it is not thread-safe.
pub static mut NAN_STRING: &str = "NaN";

/// Short infinity literal
///
/// To change the expected representation of Infinity as a string,
/// change this value during before using lexical.
pub static mut INF_STRING: &str = "inf";

/// Long infinity literal
///
/// To change the expected backup representation of Infinity as a string,
/// change this value during before using lexical.
pub static mut INFINITY_STRING: &str = "infinity";

/// Default character for scientific notation, used when the radix < 15.
///
/// To change the expected, default character for an exponent,
/// change this value during before using lexical.
pub static mut EXPONENT_DEFAULT_CHAR: u8 = b'e';

/// Backup character for scientific notation, used when the radix >= 15.
///
/// For numerical strings of radix >= 15, 'e' or 'E' is a valid digit,
/// and therefore may no longer be used as a marker for the exponent.
///
/// To change the expected, default character for an exponent,
/// change this value during before using lexical.
pub static mut EXPONENT_BACKUP_CHAR: u8 = b'^';

// CONSTANTS

// The buffer is actually a size of 60, but use 64 since it's a power of 2.
// Simple, fast optimization.
// Since we're declaring a variable on the stack, and our power-of-two
// alignment dramatically improved atoi performance, do it.
// Use 256, actually, since we seem to have memory issues with 64-bits.
// Clearly not sufficient memory allocated for non-base10 values.
pub(crate) const MAX_FLOAT_SIZE: usize = 256;
pub const BUFFER_SIZE: usize = MAX_FLOAT_SIZE;

// FUNCTIONS

/// Get the exponent notation character.
pub(crate) extern "C" fn exponent_notation_char(radix: u32)
    -> u8
{
    unsafe {
        if radix >= 15 { EXPONENT_BACKUP_CHAR } else { EXPONENT_DEFAULT_CHAR }
    }
}

// TEST
// ----

#[cfg(test)]
mod tests {
    use atof::*;
    use ftoa::*;
    use util::*;
    use util::test::*;
    use super::*;

    #[test]
    fn exponent_notation_char_test() {
        unsafe {
            assert_eq!(exponent_notation_char(2), EXPONENT_DEFAULT_CHAR);
            assert_eq!(exponent_notation_char(8), EXPONENT_DEFAULT_CHAR);
            assert_eq!(exponent_notation_char(10), EXPONENT_DEFAULT_CHAR);
            assert_eq!(exponent_notation_char(15), EXPONENT_BACKUP_CHAR);
            assert_eq!(exponent_notation_char(16), EXPONENT_BACKUP_CHAR);
            assert_eq!(exponent_notation_char(32), EXPONENT_BACKUP_CHAR);
        }
    }

    // Only enable when no other threads touch NAN_STRING or INFINITY_STRING.
    #[test]
    #[ignore]
    fn special_bytes_test() {
        let mut buffer = new_buffer();
        // Test serializing and deserializing special strings.
        assert!(try_atof32_slice(10, b"NaN").value.is_nan());
        assert!(try_atof32_slice(10, b"nan").value.is_nan());
        assert!(try_atof32_slice(10, b"NAN").value.is_nan());
        assert!(try_atof32_slice(10, b"inf").value.is_infinite());
        assert!(try_atof32_slice(10, b"INF").value.is_infinite());
        assert!(try_atof32_slice(10, b"Infinity").value.is_infinite());
        assert_eq!(f64toa_slice(f64::NAN, 10, &mut buffer), b"NaN");
        assert_eq!(f64toa_slice(f64::INFINITY, 10, &mut buffer), b"inf");

        unsafe {
            NAN_STRING = "nan";
            INF_STRING = "Infinity";
        }

        assert!(try_atof32_slice(10, b"inf").error.code == ErrorCode::InvalidDigit);
        assert!(try_atof32_slice(10, b"Infinity").value.is_infinite());
        assert_eq!(f64toa_slice(f64::NAN, 10, &mut buffer), b"nan");
        assert_eq!(f64toa_slice(f64::INFINITY, 10, &mut buffer), b"Infinity");

        unsafe {
            NAN_STRING = "NaN";
            INF_STRING = "inf";
        }
    }
}