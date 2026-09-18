use core::ops::Deref;

use super::{DshotError, DshotTelemetryFrame};

/// A captured GCR frame received straight from the PIO FIFO block.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq, PartialOrd, Ord)]
pub struct GcrFrame(u32);

impl From<GcrFrame> for u32 {
    #[inline]
    fn from(frame: GcrFrame) -> Self {
        frame.0
    }
}

impl Deref for GcrFrame {
    type Target = u32;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl GcrFrame {
    // Standard 5-bit GCR to 4-bit Nibble translation map
    const INVALID_NIBBLE: u16 = 255;
    const QUINTET_TO_NIBBLE: [u16; 32] = [
        255, 255, 255, 255, 255, 255, 255, 255, 255, 9, 10, 11, 255, 13, 14, 15, 255, 255, 2, 3, 255, 5, 6, 7, 255, 0,
        8, 1, 255, 4, 12, 255,
    ];

    #[inline]
    #[must_use]
    pub const fn from_raw(raw_pio: u32) -> Self {
        // The PIO delivers 20 bits of decoded GCR payload
        Self(raw_pio & 0x000F_FFFF)
    }

    #[inline]
    #[must_use]
    pub const fn raw_20(self) -> u32 {
        self.0
    }

    /// Decodes the 20-bit GCR stream into a standard 16-bit `Dshot` frame.
    /// # Errors
    #[inline]
    pub fn try_decode(self) -> Result<DshotTelemetryFrame, DshotError> {
        let gcr20 = self.0;

        // Extract the 5-bit quintets.
        // Because the PIO shifts LSB-first, the chronological data order is inverted:
        // The first bits received land in the highest positions (bits 15-19).
        let quintet3 = (gcr20 >> 15) & 0x1F; // First received (MSB of DShot frame)
        let quintet2 = (gcr20 >> 10) & 0x1F;
        let quintet1 = (gcr20 >> 5) & 0x1F;
        let quintet0 = gcr20 & 0x1F; // Last received (LSB of DShot frame)

        let nibble3 = Self::QUINTET_TO_NIBBLE[quintet3 as usize];
        let nibble2 = Self::QUINTET_TO_NIBBLE[quintet2 as usize];
        let nibble1 = Self::QUINTET_TO_NIBBLE[quintet1 as usize];
        let nibble0 = Self::QUINTET_TO_NIBBLE[quintet0 as usize];

        // If any translation hits an invalid code pattern (255), drop the packet
        if nibble0 == Self::INVALID_NIBBLE
            || nibble1 == Self::INVALID_NIBBLE
            || nibble2 == Self::INVALID_NIBBLE
            || nibble3 == Self::INVALID_NIBBLE
        {
            return Err(DshotError::InvalidNrziData);
        }

        // Reconstruct the original 16-bit DShot telemetry word layout
        let telemetry_word = nibble0 | (nibble1 << 4) | (nibble2 << 8) | (nibble3 << 12);

        let frame = DshotTelemetryFrame::try_from(telemetry_word)?;

        if frame.checksum_is_ok() {
            Ok(frame)
        } else {
            Err(DshotError::InvalidChecksum)
        }
    }
}

#[cfg(test)]
mod test_traits {
    use super::*;

    fn is_full_eq<T: Sized + Send + Sync + Unpin + Copy + Clone + Default + Eq + PartialEq>() {}

    #[test]
    fn normal_types() {
        is_full_eq::<GcrFrame>();
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    // Helper function to build a raw 20-bit GCR integer from four 5-bit quintets.
    // Simulates how the PIO loads them LSB-first into the register buffer.
    fn make_raw_pio_gcr(q3: u32, q2: u32, q1: u32, q0: u32) -> u32 {
        (q3 << 15) | (q2 << 10) | (q1 << 5) | q0
    }

    #[test]
    fn test_valid_zero_erpm_frame() {
        // Payload: 12-bit value = 0x000 (Nibbles: q3=0, q2=0, q1=0) -> GCR: 0x19
        // Checksum: !(0 ^ 0 ^ 0) & 0x0F = 0x0F.     (Nibble q0=0x0F) -> GCR: 0x0F

        let gcr_payload_zero = 0x19; // Decodes to 0x0
        let gcr_checksum_zero = 0x0F; // Decodes to 0x0F

        let raw_pio = make_raw_pio_gcr(gcr_payload_zero, gcr_payload_zero, gcr_payload_zero, gcr_checksum_zero);
        let frame = GcrFrame::from_raw(raw_pio);

        let decode_result = frame.try_decode();
        assert!(decode_result.is_ok(), "Expected valid zero eRPM frame decoding to succeed");
    }

    #[test]
    fn test_valid_active_erpm_frame() {
        // Simulating a moving motor with a 12-bit period telemetry value of 0x55A
        //   nibble3 = 0x5 -> GCR: 0x15
        //   nibble2 = 0x5 -> GCR: 0x15
        //   nibble1 = 0xA -> GCR: 0x0A
        // Checksum calculation: !(0x5 ^ 0x5 ^ 0xA) & 0x0F = !0xA & 0x0F = 0x5
        //   nibble0 = 0x5 -> GCR: 0x15

        let gcr_5 = 0x15;
        let gcr_a = 0x0A;

        let raw_pio = make_raw_pio_gcr(gcr_5, gcr_5, gcr_a, gcr_5);
        let frame = GcrFrame::from_raw(raw_pio);

        let decode_result = frame.try_decode();

        assert!(decode_result.is_ok(), "Expected valid active eRPM frame decoding to succeed");

        // If your test harness handles actual structural return verification,
        // you can assert the final parsed u16 word evaluates to 0x55A5:
        let decoded_frame = decode_result.unwrap();
        assert_eq!(decoded_frame.raw_16(), 0x55A5, "Decoded DShot telemetry word layout mismatch");
    }
    #[test]
    fn test_valid_edt_temperature_frame() {
        // Let's simulate an EDT Temperature frame.
        // Say the 12-bit payload is 0x24E:
        //   nibble3 = 0x2 (EDT Temperature category) -> GCR: 0x12
        //   nibble2 = 0x4 (Data high)                -> GCR: 0x1D
        //   nibble1 = 0xE (Data low)                 -> GCR: 0x0E
        // Checksum: !(0x2 ^ 0x4 ^ 0xE) = !0x8 = 0x7 -> GCR: 0x17

        let raw_pio = make_raw_pio_gcr(0x12, 0x1D, 0x0E, 0x17);
        let frame = GcrFrame::from_raw(raw_pio);

        let decode_result = frame.try_decode();
        assert!(decode_result.is_ok(), "Expected valid EDT frame to decode successfully");
    }

    #[test]
    fn test_invalid_gcr_sequence() {
        // The 5-bit value 0x00 is completely forbidden in GCR (violates run-length constraints)
        // and maps to 255 (0xFF) in our lookup array.
        let raw_pio = make_raw_pio_gcr(0x00, 0x19, 0x19, 0x19);
        let frame = GcrFrame::from_raw(raw_pio);

        let decode_result = frame.try_decode();
        assert!(
            matches!(decode_result, Err(DshotError::InvalidNrziData)),
            "Expected failure due to invalid wire patterns"
        );
    }

    #[test]
    fn test_corrupted_checksum() {
        // Payload: 12-bit value = 0x000 (GCR quintets: 0x19, 0x19, 0x19)
        // Correct checksum should decode to 0x0F (GCR: 0x0F)
        // Let's corrupt it by sending an incorrect checksum token (GCR: 0x15)

        let raw_pio = make_raw_pio_gcr(0x19, 0x19, 0x19, 0x15);
        let frame = GcrFrame::from_raw(raw_pio);

        let decode_result = frame.try_decode();
        assert!(
            matches!(decode_result, Err(DshotError::InvalidChecksum)),
            "Expected validation rejection due to checksum mismatch"
        );
    }
}
