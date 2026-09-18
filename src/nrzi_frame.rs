use core::ops::Deref;

use super::{DshotError, DshotTelemetryFrame};

/// **NRZI** stands for Non-Return-to-Zero, Inverted.
/// 21-bit edge transition NRZI.
/// Decodes to 20-bit binary GCR,
/// which then decodes to 16-bit `DshotTelemetryFrame`.
// See https://en.wikipedia.org/wiki/Run-length_limited#GCR:_(0,2)_RLL for details of the GCR encoding.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq, PartialOrd, Ord)]
pub struct NrziFrame(u32);

impl From<NrziFrame> for u32 {
    #[inline]
    fn from(frame: NrziFrame) -> Self {
        frame.0
    }
}

impl Deref for NrziFrame {
    type Target = u32;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl NrziFrame {
    // 5-bit GCR wire sequence mapped directly back to 4-bit nibbles.
    // Invalid codes are marked with 255 (0xFF).
    const QUINTET_TO_NIBBLE: [u16; 32] = [
        255, 255, 255, 255, 255, 255, 255, 255, 255, 9, 10, 11, 255, 13, 14, 15, 255, 255, 2, 3, 255, 5, 6, 7, 255, 0,
        8, 1, 255, 4, 12, 255,
    ];

    #[inline]
    #[must_use]
    pub const fn from_raw_21(raw_21: u32) -> Self {
        // Mask explicitly to 21 bits (0x1F_FFFF) to preserve the full physical packet capacity
        Self(raw_21 & 0x001F_FFFF)
    }

    #[inline]
    #[must_use]
    pub const fn raw_21(self) -> u32 {
        self.0
    }

    /// Check if checksum is ok (XOR of all 4 nibbles must equal 0x0F).
    /// Fast validation check on the raw telemetry framework layout.
    #[inline]
    #[must_use]
    pub const fn is_valid(self) -> bool {
        let value = self.0;
        let checksum = (value ^ (value >> 4) ^ (value >> 8) ^ (value >> 12)) & 0x0F;
        checksum == 0x0F
    }

    /// Converts the physical 21-bit NRZI transition buffer into a clean 20-bit GCR token.
    /// Handles chronological line boundaries accurately by tracking state deltas.
    #[inline]
    #[must_use]
    const fn nrzi21_to_gcr20(value: u32) -> u32 {
        let mut gcr_output: u32 = 0;

        // DShot telemetry packets are transmitted MSB-first.
        // Bit 20 is the leading sync zero, establishing our initial wire level.
        let mut previous_state = (value >> 20) & 0x01;

        // Sequentially parse down through the remaining 20 bits of data payload
        let mut ii = 19;
        loop {
            let current_state = (value >> ii) & 0x01;

            // NRZI Decoder Core Rule: A state transition means 1, no change means 0.
            let decoded_bit = current_state ^ previous_state;
            gcr_output = (gcr_output << 1) | decoded_bit;

            previous_state = current_state;

            if ii == 0 {
                break;
            }
            ii -= 1;
        }

        gcr_output
    }

    /// Maps the unified 20-bit GCR token into a standard 16-bit payload using the conversion matrix.
    #[inline]
    fn gcr20_to_erpm(gcr20: u32) -> Result<DshotTelemetryFrame, DshotError> {
        let nibble0 = Self::QUINTET_TO_NIBBLE[(gcr20 & 0x1F) as usize];
        let nibble1 = Self::QUINTET_TO_NIBBLE[((gcr20 >> 5) & 0x1F) as usize];
        let nibble2 = Self::QUINTET_TO_NIBBLE[((gcr20 >> 10) & 0x1F) as usize];
        let nibble3 = Self::QUINTET_TO_NIBBLE[((gcr20 >> 15) & 0x1F) as usize];

        if nibble0 == 0xFF || nibble1 == 0xFF || nibble2 == 0xFF || nibble3 == 0xFF {
            return Err(DshotError::InvalidNrziData);
        }

        let erpm_raw = nibble0 | (nibble1 << 4) | (nibble2 << 8) | (nibble3 << 12);
        // `try_from` will fail if the checksum is invalid.
        DshotTelemetryFrame::try_from(erpm_raw)
    }

    /// Public processing method to ingest raw incoming wire metrics.
    /// # Errors
    #[inline]
    pub fn try_decode(self) -> Result<DshotTelemetryFrame, DshotError> {
        // Optimization: Execute the fast raw validation check first.
        // If line noise corrupted the layout, reject it immediately before calculating GCR lookups.
        if !self.is_valid() {
            return Err(DshotError::InvalidChecksum);
        }

        let gcr20 = Self::nrzi21_to_gcr20(self.0);
        let erpm_telemetry_frame = Self::gcr20_to_erpm(gcr20)?;
        if erpm_telemetry_frame.checksum_is_ok() {
            Ok(erpm_telemetry_frame)
        } else {
            Err(DshotError::InvalidChecksum)
        }
    }
}

#[allow(unused)]
impl NrziFrame {
    const NRZI_BIT_LENGTHS: [u32; 17] = [0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 5, 5, 5];
    const NRZI_SET_BITS: [u32; 6] = [0b_00000, 0b_00001, 0b_00011, 0b_00111, 0b_01111, 0b_11111];

    /// # Errors
    #[inline]
    fn nrzi21_to_erpm(nrzi21: u32) -> Result<DshotTelemetryFrame, DshotError> {
        Self::gcr20_to_erpm(Self::nrzi21_to_gcr20(nrzi21))
    }

    /// Decode samples returned by Raspberry Pi PIO implementation.
    /// 64-bit value gives 3x oversampling of NRZI21 code.
    ///
    /// Returns the value of the Extended Dshot Telemetry (EDT) frame (without the checksum).
    /// # Errors `DshotError`
    pub fn decode_samples(value: u64) -> Result<DshotTelemetryFrame, DshotError> {
        // telemetry data must start with a 0, so if the first bit is high, we don't have any data
        if (value & 0x8000_0000_0000_0000) != 0 {
            return Err(DshotError::NoGcrData);
        }

        let mut consecutive_bit_count: usize = 1; // we always start with the MSB
        let mut current_bit: u32 = 0;
        let mut bit_count: u32 = 0;
        let mut nrzi21_data: u32 = 0;

        // starting at 2nd bit since we know our data starts with a 0
        // 56 samples @ 0.917us sample rate = 51.33us sampled
        // loop the mask from 2nd MSB to  LSB
        let mut mask: u64 = 0x4000_0000_0000_0000;
        while mask != 0 {
            if ((value & mask) != 0) == (current_bit != 0) {
                // if the masked bit match the current string of bits then increment consecutive_bit_count.
                consecutive_bit_count += 1;
                if consecutive_bit_count > 16 {
                    // invalid run length at the current sample rate (outside of GCR_BIT_LENGTHS table)
                    return Err(DshotError::InvalidRunLength);
                }
            } else {
                // if the masked bit doesn't match the current string of bits then end the current string and flip current_bit
                // bitshift gcr_result by N
                nrzi21_data <<= Self::NRZI_BIT_LENGTHS[consecutive_bit_count];
                // and then set N bits in gcr_result, if current_bit is 1
                if current_bit != 0 {
                    nrzi21_data |= Self::NRZI_SET_BITS[Self::NRZI_BIT_LENGTHS[consecutive_bit_count] as usize];
                }
                bit_count += Self::NRZI_BIT_LENGTHS[consecutive_bit_count];
                // invert current_bit, and reset consecutive_bit_count
                current_bit = !current_bit;
                consecutive_bit_count = 1; // first bit found in the string is the one we just processed
            }
            mask >>= 1;
        }

        // outside the loop, we still need to account for the final bits if the string ends with 1s
        // bitshift gcr_result by N, and
        nrzi21_data <<= Self::NRZI_BIT_LENGTHS[consecutive_bit_count];
        // then set set N bits in gcr_result, if current_bit is 1
        if current_bit != 0 {
            nrzi21_data |= Self::NRZI_SET_BITS[Self::NRZI_BIT_LENGTHS[consecutive_bit_count] as usize];
        }
        // count bit_count (for debugging)
        bit_count += Self::NRZI_BIT_LENGTHS[consecutive_bit_count];

        // NRZI data should be 21 bits
        if bit_count < 21 {
            return Err(DshotError::InvalidNrziData);
        }

        // chop the GCR data down to just the 21 most significant bits
        nrzi21_data >>= bit_count - 21;

        // convert 21-bit edge transition NRZI to 20-bit binary GCR
        let erpm_telemetry_frame = Self::nrzi21_to_erpm(nrzi21_data)?;

        Ok(erpm_telemetry_frame)
    }
}

#[cfg(test)]
mod test_traits {
    use super::*;

    fn is_full<T: Sized + Send + Sync + Unpin + Copy + Clone + Default + PartialEq>() {}

    #[test]
    fn normal_types() {
        is_full::<NrziFrame>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gcr_decode_rejects_invalid_input() {
        // All zeros and all ones should fail
        assert_eq!(Err(DshotError::InvalidChecksum), NrziFrame::from_raw_21(0).try_decode());
        assert_eq!(Err(DshotError::InvalidChecksum), NrziFrame::from_raw_21(0x1FFFF).try_decode());
    }
    #[test]
    fn valid() {
        assert!(NrziFrame::from_raw_21(0xF000).is_valid()); // 0^0^0^F = F ✓
        assert!(NrziFrame::from_raw_21(0x8421).is_valid()); // 1^2^4^8 = F ✓
    }

    #[test]
    fn invalid() {
        assert!(!NrziFrame::from_raw_21(0x1234).is_valid()); // 4^3^2^1 = 4 ✗
        assert!(!NrziFrame::from_raw_21(0x0000).is_valid()); // 0^0^0^0 = 0 ✗
    }
}
