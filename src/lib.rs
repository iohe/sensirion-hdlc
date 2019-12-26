//! # sensirion-hdlc
//! Only frames the data.  Rust implementation of Sensirion High-level Data Link Control (HDLC)
//! library.
//!
//! ## Usage
//!
//! ### Encode packet
//! ```rust
//! extern crate sensirion_hdlc;
//! use sensirion_hdlc::{SpecialChars, encode};
//!
//! let msg = [0x01, 0x50, 0x00, 0x00, 0x00, 0x05, 0x80, 0x09, 0x20];
//! let cmp = [0x7E, 0x01, 0x50, 0x00, 0x00, 0x00, 0x05, 0x80, 0x09, 0x20, 0x7E];
//!
//! let result = encode(&msg, SpecialChars::default()).unwrap();
//!
//! assert_eq!(result[0..result.len()], cmp);
//! ```
//!
//! ### Custom Special Characters
//! ```rust
//! extern crate sensirion_hdlc;
//! use sensirion_hdlc::{SpecialChars, encode};
//!
//! let msg = [0x01, 0x7E, 0x70, 0x50, 0x00, 0x05, 0x80, 0x09, 0xe2];
//! let cmp = [0x71, 0x01, 0x7E, 0x70, 0x50, 0x50, 0x00, 0x05, 0x80, 0x09, 0xe2, 0x71];
//! let chars = SpecialChars::new(0x71, 0x70, 0x51, 0x50, 0x11, 0x31, 0x13, 0x33).unwrap();
//!
//! let result = encode(&msg, chars).unwrap();
//!
//! assert_eq!(result[0..result.len()], cmp)
//! ```
//!
//! ### Decode packet
//! ```rust
//! extern crate sensirion_hdlc;
//! use sensirion_hdlc::{SpecialChars, decode};
//!
//! let chars = SpecialChars::default();
//! let msg = [
//!     chars.fend, 0x01, 0x50, 0x00, 0x00, 0x00, 0x05, 0x80, 0x29, chars.fend,
//! ];
//! let cmp = [0x01, 0x50, 0x00, 0x00, 0x00, 0x05, 0x80, 0x29];
//!
//! let result = decode(&msg, chars).unwrap();
//!
//! assert_eq!(result[0..result.len()], cmp);
//! ```

#![no_std]
#![deny(missing_docs)]

use arrayvec::ArrayVec;

/// Special Character structure for holding the encode and decode values.
///
/// # Default
///
/// * **FEND**  = 0x7E;
/// * **FESC**  = 0x7D;
/// * **TFEND** = 0x5E;
/// * **TFESC** = 0x5D;
/// * **OB1**   = 0x11;
/// * **TFOB1** = 0x31;
/// * **OB2**   = 0x13;
/// * **TFOB2** = 0x33;
#[derive(Debug, Copy, Clone)]
pub struct SpecialChars {
    /// Frame END. Byte that marks the beginning and end of a packet
    pub fend: u8,
    /// Frame ESCape. Byte that marks the start of a swap byte
    pub fesc: u8,
    /// Trade Frame END. Byte that is substituted for the FEND byte
    pub tfend: u8,
    /// Trade Frame ESCape. Byte that is substituted for the FESC byte
    pub tfesc: u8,
    /// Original Byte 1. Byte that will be substituted for the TFOB1 byte
    pub ob1: u8,
    /// Trade Frame ESCape. Byte that is substituted for the Original Byte 1
    pub tfob1: u8,
    /// Original Byte 2. Byte that is substituted for the TFOB2 byte
    pub ob2: u8,
    /// Trade Frame Origina Byte 2. Byte that is substituted for the Original Byte 2
    pub tfob2: u8,
}

impl Default for SpecialChars {
    /// Creates the default SpecialChars structure for encoding/decoding a packet
    fn default() -> SpecialChars {
        SpecialChars {
            fend: 0x7E,
            fesc: 0x7D,
            tfend: 0x5E,
            tfesc: 0x5D,
            ob1: 0x11,
            tfob1: 0x31,
            ob2: 0x13,
            tfob2: 0x33,
        }
    }
}
impl SpecialChars {
    /// Creates a new SpecialChars structure for encoding/decoding a packet
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        fend: u8,
        fesc: u8,
        tfend: u8,
        tfesc: u8,
        ob1: u8,
        tfob1: u8,
        ob2: u8,
        tfob2: u8,
    ) -> Result<SpecialChars, HDLCError> {
        // Safety check to make sure the special character values are all unique
        let values: [u8; 8] = [fend, fesc, tfend, tfesc, ob1, tfob1, ob2, tfob2];
        for i in 0..values.len() - 1 {
            if values[i + 1..].contains(&values[i]) {
                return Err(HDLCError::DuplicateSpecialChar);
            }
        }

        Ok(SpecialChars {
            fend,
            fesc,
            tfend,
            tfesc,
            ob1,
            tfob1,
            ob2,
            tfob2,
        })
    }
}

/// Produces escaped (encoded) message surrounded with `FEND`
///
/// # Inputs
/// * **Vec<u8>**: A vector of the bytes you want to encode
/// * **SpecialChars**: The special characters you want to swap
///
/// # Output
///
/// * **Result<ArrayVec<u8;1024>>**: Encoded output message
///
///
/// # Error
///
/// * **HDLCError::TooMuchData**: More than 260 bytes to be encoded
///
/// # Example
/// ```rust
/// extern crate sensirion_hdlc;
/// let chars = sensirion_hdlc::SpecialChars::default();
/// let input: Vec<u8> = vec![0x01, 0x50, 0x00, 0x00, 0x00, 0x05, 0x80, 0x09];
/// let op_vec = sensirion_hdlc::encode(&input.to_vec(), chars);
/// ```
pub fn encode(data: &[u8], s_chars: SpecialChars) -> Result<ArrayVec<[u8; 1024]>, HDLCError> {
    if data.len() > 260 {
        return Err(HDLCError::TooMuchData);
    }

    // Iterator over the input that allows peeking
    let input_iter = data.iter();

    let mut output = ArrayVec::<[_; 1024]>::new();
    //Push initial FEND
    output.push(s_chars.fend);

    // Loop over every byte of the message
    for value in input_iter {
        match *value {
            // FEND , FESC, ob1 and ob2
            val if val == s_chars.fesc => {
                output.push(s_chars.fesc);
                output.push(s_chars.tfesc);
            }
            val if val == s_chars.fend => {
                output.push(s_chars.fesc);
                output.push(s_chars.tfend);
            }
            val if val == s_chars.ob1 => {
                output.push(s_chars.fesc);
                output.push(s_chars.tfob1);
            }
            val if val == s_chars.ob2 => {
                output.push(s_chars.fesc);
                output.push(s_chars.tfob2);
            }
            // Handle any other bytes
            _ => output.push(*value),
        }
    }

    // Push final FEND
    output.push(s_chars.fend);

    Ok(output)
}

/// Produces unescaped (decoded) message without `FEND` characters.
///
/// # Inputs
/// * **Vec<u8>**: A vector of the bytes you want to decode
/// * **SpecialChars**: The special characters you want to swap
///
/// # Output
///
/// * **Result<ArrayVec<u8;1024>>**: Decoded output message
///
/// # Error
///
/// * **HDLCError::TooMuchDecodedData**: More than expected 260 bytes of decoded data
/// * **HDLCError::FendCharInData**: Checks to make sure the full decoded message is the full
/// length.  Found the `SpecialChars::fend` inside the message.
/// * **HDLCError::MissingTradeChar**: Checks to make sure every frame escape character `fesc`
/// is followed by either a `tfend` or a `tfesc`.
/// * **HDLCError::MissingFirstFend**: Input vector is missing a first `SpecialChars::fend`
/// * **HDLCError::MissingFinalFend**: Input vector is missing a final `SpecialChars::fend`
/// * **HDLCError::TooFewData**: Data to decode is fewer than 4 bytes`
/// * **HDLCError::TooMuchData**: Data to decode is larger than 1000 bytes`
///
///
/// # Example
/// ```rust
/// extern crate sensirion_hdlc;
/// let chars = sensirion_hdlc::SpecialChars::default();
/// let input =[ 0x7E, 0x01, 0x50, 0x00, 0x00, 0x00, 0x05, 0x80, 0x09, 0x7E];
/// let op_vec = sensirion_hdlc::decode(&input.to_vec(), chars);
/// ```
pub fn decode(input: &[u8], s_chars: SpecialChars) -> Result<ArrayVec<[u8; 1024]>, HDLCError> {
    if input.len() < 4 {
        return Err(HDLCError::TooFewData);
    }

    if input.len() > 1000 {
        return Err(HDLCError::TooMuchData);
    }

    // Verify input begins with a FEND
    if input[0] != s_chars.fend {
        return Err(HDLCError::MissingFirstFend);
    }
    // Verify input ends with a FEND
    if input[input.len() - 1] != s_chars.fend {
        return Err(HDLCError::MissingFinalFend);
    }

    let mut output = ArrayVec::<[u8; 1024]>::new();

    // Iterator over the input that allows peeking
    let mut input_iter = input[1..input.len() - 1].iter().peekable();

    // Loop over every byte of the message
    while let Some(value) = input_iter.next() {
        match *value {
            // Handle a FESC
            val if val == s_chars.fesc => match input_iter.next() {
                Some(&val) if val == s_chars.tfend => output.push(s_chars.fend),
                Some(&val) if val == s_chars.tfesc => output.push(s_chars.fesc),
                Some(&val) if val == s_chars.tfob1 => output.push(s_chars.ob1),
                Some(&val) if val == s_chars.tfob2 => output.push(s_chars.ob2),
                _ => return Err(HDLCError::MissingTradeChar),
            },
            // Handle a FEND
            val if val == s_chars.fend => {
                return Err(HDLCError::FendCharInData);
            }
            // Handle any other bytes
            _ => output.push(*value),
        }
    }

    if output.len() > 260 {
        return Err(HDLCError::TooMuchDecodedData);
    }

    Ok(output)
}

#[derive(Debug, PartialEq)]
/// Common error for HDLC actions.
pub enum HDLCError {
    /// Catches duplicate special characters.   
    DuplicateSpecialChar,
    /// Catches a random sync char in the data.
    FendCharInData,
    /// Catches a random swap char, `fesc`, in the data with no `tfend` or `tfesc`.
    MissingTradeChar,
    /// No first fend on the message.    
    MissingFirstFend,
    /// No final fend on the message.
    MissingFinalFend,
    /// Too much data to be converted into a SHDLC frame
    TooMuchData,
    /// Too few data to be converted from a SHDLC frame
    TooFewData,
    /// Checksum for decoded Frame is invalid
    InvalidChecksum,
    /// More than 259 bytes resulted after decoding SHDLC frame
    TooMuchDecodedData,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_start_measumement() {
        let mosi_data = [0x00, 0x00, 0x02, 0x01, 0x03, 0xf9];
        let expected = [0x7e, 0x00, 0x00, 0x02, 0x01, 0x03, 0xf9, 0x7e];
        let encoded = encode(&mosi_data, SpecialChars::default()).unwrap();
        assert_eq!(encoded[0..encoded.len()], expected);
    }

    #[test]
    fn encode_test() {
        let mosi_data = [0x00, 0x01, 0x00, 0xfe];
        let expected = [0x7e, 0x00, 0x01, 0x00, 0xfe, 0x7e];
        let encoded = encode(&mosi_data, SpecialChars::default()).unwrap();
        assert_eq!(encoded[0..encoded.len()], expected);
    }

    #[test]
    fn decode_test() {
        let expected = [0x00, 0x01, 0x00, 0xfe];
        let mosi_data = [0x7e, 0x00, 0x01, 0x00, 0xfe, 0x7e];
        let encoded = decode(&mosi_data, SpecialChars::default()).unwrap();
        assert_eq!(encoded[0..encoded.len()], expected);
    }
}
