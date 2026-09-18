use core::ops::Deref;

use super::DshotCommand;

/// `DshotCommandFrame`: transmitted from the Flight Controller(FC) to the ESC.
///
/// Whenever the FC wants the motor to spin, beep, or change direction, it transmits a 16-bit `DshotCommandFrame`.<br>
/// Bits 0–10 (11 bits): Throttle value or Command, values 48 to 2047 for motor speed (throttle), values 1 to 47 for commands.<br>
/// Bit 11     (1 bit):  Telemetry Request Flag.<br>
/// Bits 12–15 (4 bits): Checksum (technically a 4-bit Longitudinal Redundancy Check (LRC)).<br>
///
/// | Operational Aspect     | Unidirectional (Throttle)           | Unidirectional (Commands)                 | Bidirectional (Throttle & Commands)                |
/// | :--------------------- | :---------------------------------- | :---------------------------------------- | :------------------------------------------------- |
/// | **Telemetry Bit**      | **`false`** (Set to `0`)            | **`true`** (Set to `1`)                   | **`true`** (Set to `1`)                            |
/// | **XOR Checksum Mode**  | **Standard**                        | **Bitwise Inverted**                      | **Bitwise Inverted**                               |
/// | **ESC Action**         | Executes throttle<br>Remains silent | Executes command<br>Returns a ghost reply | Executes command<br>Returns a telemetry frame      |
/// | **FC Pin Mode**        | Permanent **Output**                | Permanent **Output**                      | Flips from **Output to Input** right after TX      |
/// | **Repetition Gate**    | Streams continuously                | **Must repeat ~10 times** to execute      | Commands **must repeat ~10 times** to execute      |
/// | **FC Software Action** | Fire-and-forget stream              | Fire-and-forget stream                    | Transmits, then pauses ~30µs to capture `GcrFrame` |
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq, PartialOrd, Ord)]
pub struct DshotCommandFrame(u16);

impl TryFrom<u16> for DshotCommandFrame {
    type Error = u16;

    #[inline]
    fn try_from(value: u16) -> Result<Self, u16> {
        if value <= Self::MAX_RAW_VALUE {
            Ok(DshotCommandFrame::encode_raw(value, DshotCommandFrame::NO_TELEMETRY))
        } else {
            Err(value)
        }
    }
}

impl From<DshotCommandFrame> for u16 {
    #[inline]
    fn from(frame: DshotCommandFrame) -> Self {
        frame.raw()
    }
}

impl Deref for DshotCommandFrame {
    type Target = u16;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DshotCommandFrame {
    pub const NO_TELEMETRY: bool = false;
    pub const WITH_TELEMETRY: bool = true;

    pub const UNI_DIRECTIONAL: bool = false;
    pub const BI_DIRECTIONAL: bool = true;

    // `Dshot` command/throttle payload is 11 bits (0 to 2047)
    pub const MAX_RAW_VALUE: u16 = 2047;
    pub const THROTTLE_OFFSET: u16 = 48;
    pub const THROTTLE_MIN: u16 = 48;
    pub const THROTTLE_MAX: u16 = 2047;

    const TELEMETRY_BIT: u16 = 0x10;
    const CHECKSUM_BITS: u16 = 0x0F;

    // 4-bit to 5-bit GCR translation table.
    pub(crate) const NIBBLE_TO_QUINTET: [u8; 16] =
        [0x19, 0x1B, 0x12, 0x13, 0x1D, 0x15, 0x16, 0x17, 0x1A, 0x09, 0x0A, 0x0B, 0x1E, 0x0D, 0x0E, 0x0F];

    #[inline]
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self::encode_raw(value, Self::NO_TELEMETRY)
    }

    #[inline]
    #[must_use]
    pub const fn from_raw(value: u16) -> Self {
        Self(value)
    }

    #[inline]
    #[must_use]
    pub const fn from_command(command: DshotCommand) -> Self {
        // commands set the telemetry bit whether in unidirectional or bidirectional mode.
        Self::encode_raw(command as u16, Self::WITH_TELEMETRY)
    }

    #[inline]
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }

    /// Extracts the original 11-bit command/throttle value from the processed 16-bit frame.
    #[inline]
    #[must_use]
    pub const fn value(self) -> u16 {
        self.0 >> 5
    }

    #[inline]
    #[must_use]
    pub const fn is_telemetry_enabled(self) -> bool {
        (self.0 & Self::TELEMETRY_BIT) != 0
    }

    #[inline]
    #[must_use]
    pub const fn checksum(self) -> u16 {
        self.0 & Self::CHECKSUM_BITS
    }

    /// Calculates the standard 4-bit XOR outbound checksum.
    #[inline]
    #[must_use]
    pub const fn calculate_checksum(frame_raw: u16) -> u16 {
        (frame_raw ^ (frame_raw >> 4) ^ (frame_raw >> 8)) & 0x0F
    }

    /// Assembles an 11-bit command and a telemetry flag into a complete 16-bit `Dshot` transmission word.
    #[must_use]
    pub const fn encode_raw(value: u16, with_telemetry: bool) -> Self {
        // Clamp input value to prevent register overflow corruption
        let value = if value > Self::MAX_RAW_VALUE { Self::MAX_RAW_VALUE } else { value };

        // Shift left by 1 and inject the telemetry selection bit
        let frame_raw = if with_telemetry { (value << 1) | 0x01 } else { value << 1 };

        // Calculate the base XOR checksum
        let mut checksum = Self::calculate_checksum(frame_raw);

        // Both Unidirectional and Bidirectional DShot require the checksum to be inverted when the telemetry bit is set.
        if with_telemetry {
            checksum = (!checksum) & 0x0F;
        }
        Self((frame_raw << 4) | checksum)
    }

    /// Converts a throttle scale `[0.0, 1.0]` directly to the `Dshot` frame range `[48, 2047]`.
    /// In unidirectional mode the telemetry bit is not set and the checksum is not inverted.
    #[must_use]
    pub fn from_throttle_unidirectional(throttle: f32) -> Self {
        #[allow(unused)]
        use num_traits::float::FloatCore;

        // Clamp throttle to prevent out-of-bounds calculations
        let throttle = throttle.clamp(0.0, 1.0);

        // Scale linearly across the available 1999 active throttle steps
        let range = f32::from(Self::THROTTLE_MAX - Self::THROTTLE_MIN);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let dshot_value = (throttle * range).round() as u16 + Self::THROTTLE_MIN;

        // For unidirectional frames the telemetry bit IS NOT set.
        Self::encode_raw(dshot_value, Self::NO_TELEMETRY)
    }

    /// Converts a throttle scale `[0.0, 1.0]` directly to the `Dshot` frame range `[48, 2047]`.
    /// In bidirectional mode the telemetry bit is not set and the checksum is not inverted.
    #[must_use]
    pub fn from_throttle_bidirectional(throttle: f32) -> Self {
        #[allow(unused)]
        use num_traits::float::FloatCore;

        // Clamp throttle to prevent out-of-bounds calculations
        let throttle = throttle.clamp(0.0, 1.0);

        // Scale linearly across the available 1999 active throttle steps
        let range = f32::from(Self::THROTTLE_MAX - Self::THROTTLE_MIN);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let dshot_value = (throttle * range).round() as u16 + Self::THROTTLE_MIN;

        // For bidirectional frames the telemetry bit IS set.
        Self::encode_raw(dshot_value, Self::WITH_TELEMETRY)
    }

    #[inline]
    #[must_use]
    pub fn from_throttle(throttle: f32, bidirectional: bool) -> Self {
        if bidirectional {
            Self::from_throttle_bidirectional(throttle)
        } else {
            Self::from_throttle_unidirectional(throttle)
        }
    }
}

impl DshotCommandFrame {
    // see [DSHOT - the missing Handbook](https://brushlesswhoop.com/dshot-and-bidirectional-dshot/)
    // for a good description of these conversions
    #[inline]
    #[must_use]
    pub fn to_gcr20(self) -> u32 {
        let value = self.0;
        let mut ret = u32::from(Self::NIBBLE_TO_QUINTET[(value & 0x0F) as usize]);
        ret |= u32::from(Self::NIBBLE_TO_QUINTET[((value >> 4) & 0x0F) as usize]) << 5;
        ret |= u32::from(Self::NIBBLE_TO_QUINTET[((value >> 8) & 0x0F) as usize]) << 10;
        ret |= u32::from(Self::NIBBLE_TO_QUINTET[((value >> 12) & 0x0F) as usize]) << 15;
        ret
    }

    /// Map the GCR to a 21-bit NRZI value, this new value starts with a 0 and the rest of the bits are set by the following two rules:
    ///    1. If the current input bit in GCR data is a 1 then the output bit is the inverse of the previous output bit
    ///    2. If the current input bit in GCR data is a 0 then the output bit is the same as the previous output
    #[must_use]
    pub fn gcr20_to_nrzi21(input: u32) -> u32 {
        let mut ret = 0;
        let mut prev_gcr_bit = 0;
        let mut mask = 1 << 19;

        while mask != 0 {
            ret <<= 1;
            let input_bit = u32::from((input & mask) != 0);
            let gcr_bit = input_bit ^ prev_gcr_bit;
            prev_gcr_bit = gcr_bit;
            ret |= gcr_bit;
            mask >>= 1;
        }
        ret
    }

    #[must_use]
    pub fn gcr_encode(self) -> u32 {
        let gcr20 = self.to_gcr20();
        Self::gcr20_to_nrzi21(gcr20)
    }
}

#[cfg(test)]
mod test_traits {
    use super::*;

    fn is_full<T: Sized + Send + Sync + Unpin + Copy + Clone + Default + PartialEq>() {}

    #[test]
    fn normal_types() {
        is_full::<DshotCommandFrame>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum() {
        assert_eq!(DshotCommandFrame::calculate_checksum(0b_1000_0010_1100), 0b_0000_0000_0110,);
    }
    #[test]
    fn throttle_unidirectional() {
        let frame = DshotCommandFrame::from_throttle_unidirectional(0.0);
        assert_eq!(48, frame.value());
        assert!(!frame.is_telemetry_enabled());

        let frame = DshotCommandFrame::from_throttle_unidirectional(0.25);
        assert_eq!(548, frame.value());
        assert!(!frame.is_telemetry_enabled());

        let frame = DshotCommandFrame::from_throttle_unidirectional(0.50);
        assert_eq!(1048, frame.value());
        assert!(!frame.is_telemetry_enabled());

        let frame = DshotCommandFrame::from_throttle_unidirectional(0.75);
        assert_eq!(1547, frame.value());
        assert!(!frame.is_telemetry_enabled());

        let frame = DshotCommandFrame::from_throttle_unidirectional(1.00);
        assert_eq!(2047, frame.value());
        assert!(!frame.is_telemetry_enabled());
    }
    #[test]
    fn throttle_bidirectional() {
        let frame = DshotCommandFrame::from_throttle_bidirectional(0.0);
        assert_eq!(48, frame.value());
        assert!(frame.is_telemetry_enabled());

        let frame = DshotCommandFrame::from_throttle_bidirectional(0.25);
        assert_eq!(548, frame.value());
        assert!(frame.is_telemetry_enabled());

        let frame = DshotCommandFrame::from_throttle_bidirectional(0.50);
        assert_eq!(1048, frame.value());
        assert!(frame.is_telemetry_enabled());

        let frame = DshotCommandFrame::from_throttle_bidirectional(0.75);
        assert_eq!(1547, frame.value());
        assert!(frame.is_telemetry_enabled());

        let frame = DshotCommandFrame::from_throttle_bidirectional(1.00);
        assert_eq!(2047, frame.value());
        assert!(frame.is_telemetry_enabled());
    }
    #[test]
    fn throttle() {
        let frame = DshotCommandFrame::from_throttle(0.25, DshotCommandFrame::UNI_DIRECTIONAL);
        assert_eq!(548, frame.value());
        assert!(!frame.is_telemetry_enabled());

        let frame = DshotCommandFrame::from_throttle(0.75, DshotCommandFrame::UNI_DIRECTIONAL);
        assert_eq!(1547, frame.value());
        assert!(!frame.is_telemetry_enabled());

        let frame = DshotCommandFrame::from_throttle(0.25, DshotCommandFrame::BI_DIRECTIONAL);
        assert_eq!(548, frame.value());
        assert!(frame.is_telemetry_enabled());

        let frame = DshotCommandFrame::from_throttle(0.75, DshotCommandFrame::BI_DIRECTIONAL);
        assert_eq!(1547, frame.value());
        assert!(frame.is_telemetry_enabled());
    }
    #[rustfmt::skip]
    #[test]
    fn commands() {
        let frame = DshotCommandFrame::from_command(DshotCommand::Beep1);
        assert_eq!(0b_0000_0000_0011_1100, frame.raw());

        let frame = DshotCommandFrame::from_command(DshotCommand::SignalLineErpmTelemetry);
        assert_eq!(0b_0000_0101_1101_0111, frame.raw());

        let frame = DshotCommandFrame::from_command(DshotCommand::SignalLineErpmPeriodTelemetry);
        assert_eq!(0b_0000_0101_1111_0101, frame.raw());
    }
}
#[cfg(test)]
mod command_frame_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn test_encode_raw_no_telemetry() {
        // Use a standard throttle value: 1000
        // 1. Shift left by 1 for telemetry bit (0): 1000 << 1 = 2000 (0x7D0)
        // 2. Calculate checksum:
        //    nibble0 = 0x0, nibble1 = 0xD, nibble2 = 0x7
        //    0x0 ^ 0xD ^ 0x7 = 0xA
        // 3. Shift payload left by 4 and add checksum: (2000 << 4) | 0xA = 32010 (0x7D0A)
        let frame = DshotCommandFrame::encode_raw(1000, DshotCommandFrame::NO_TELEMETRY);

        assert_eq!(frame.raw(), 0x7D0A);
        assert_eq!(frame.value(), 1000);
        assert!(!frame.is_telemetry_enabled());
        assert_eq!(frame.checksum(), 0x0A);
    }

    #[test]
    fn test_encode_raw_with_telemetry() {
        // Use the same throttle value (1000) but enable bidirectional telemetry
        // 1. Shift left by 1 and inject telemetry bit (1): (1000 << 1) | 1 = 2001 (0x7D1)
        // 2. Calculate checksum:
        //    nibble0 = 0x1, nibble1 = 0xD, nibble2 = 0x7
        //    0x1 ^ 0xD ^ 0x7 = 0xB
        // 3. Because with_telemetry is true, encode_raw bitwise inverts this checksum.
        //    !0xB & 0x0F = !0b1011 & 0x0F = 0b0100 = 0x4
        // 4. Shift payload left by 4 and add checksum: (2001 << 4) | 0x4 = 32016 + 4 = 32020 (0x7D14)
        let frame = DshotCommandFrame::encode_raw(1000, DshotCommandFrame::WITH_TELEMETRY);

        assert_eq!(frame.raw(), 0x7D14);
        assert_eq!(frame.value(), 1000);
        assert!(frame.is_telemetry_enabled());
        assert_eq!(frame.checksum(), 0x04);
    }

    #[test]
    fn test_try_from_valid_and_invalid() {
        let valid_res = DshotCommandFrame::try_from(0);
        assert!(valid_res.is_ok());
        assert_eq!(valid_res.unwrap().value(), 0);

        let valid_res = DshotCommandFrame::try_from(1);
        assert!(valid_res.is_ok());
        assert_eq!(valid_res.unwrap().value(), 1);

        // Max valid raw value is 2047
        let valid_res = DshotCommandFrame::try_from(2047);
        assert!(valid_res.is_ok());
        assert_eq!(valid_res.unwrap().value(), 2047);

        // Over the limit should return the error value
        let invalid_res = DshotCommandFrame::try_from(2048);
        assert!(invalid_res.is_err());
        assert_eq!(invalid_res.unwrap_err(), 2048);
    }

    #[test]
    fn from_command() {
        // Using MotorStop (value 0)
        let frame = DshotCommandFrame::from_command(DshotCommand::MotorStop);
        assert_eq!(frame.value(), 0);
        assert!(frame.is_telemetry_enabled());

        // Using BeepTone1 (value 1)
        let frame_telemetry = DshotCommandFrame::from_command(DshotCommand::Beep1);
        assert_eq!(frame_telemetry.value(), 1);
        assert!(frame_telemetry.is_telemetry_enabled());
    }

    #[test]
    fn from_throttle_scaling() {
        // 1. Minimum Active Throttle (0.0) -> Should map to THROTTLE_MIN (48)
        let frame_min = DshotCommandFrame::from_throttle_unidirectional(0.0);
        assert_eq!(frame_min.value(), 48);
        let frame_min = DshotCommandFrame::from_throttle_bidirectional(0.0);
        assert_eq!(frame_min.value(), 48);

        // 2. Maximum Active Throttle (1.0) -> Should map to THROTTLE_MAX (2047)
        let frame_max = DshotCommandFrame::from_throttle_unidirectional(1.0);
        assert_eq!(frame_max.value(), 2047);
        let frame_max = DshotCommandFrame::from_throttle_bidirectional(1.0);
        assert_eq!(frame_max.value(), 2047);

        // 3. Midpoint Throttle (0.5) -> (2047 - 48) * 0.5 = 999.5 -> rounded to 1000 -> + 48 = 1048
        let frame_midpoint = DshotCommandFrame::from_throttle_unidirectional(0.5);
        assert_eq!(frame_midpoint.value(), 1048);
        let frame_midpoint = DshotCommandFrame::from_throttle_bidirectional(0.5);
        assert_eq!(frame_midpoint.value(), 1048);
    }

    #[test]
    fn from_throttle_clamping() {
        // Negative inputs should be clamped safely to 0.0 -> evaluating to 48
        let frame_neg = DshotCommandFrame::from_throttle_unidirectional(-0.25);
        assert_eq!(frame_neg.value(), 48);

        // Over-unity inputs should be clamped safely to 1.0 -> evaluating to 2047
        let frame_over = DshotCommandFrame::from_throttle_unidirectional(1.5);
        assert_eq!(frame_over.value(), 2047);
    }

    #[test]
    fn input_value_clamping_protection() {
        // If an absolute rogue value over 2047 bypasses validation into encode_raw directly,
        // the constructor must clamp it to 2047 instead of shifting out-of-bounds junk bits
        let frame = DshotCommandFrame::encode_raw(9999, DshotCommandFrame::NO_TELEMETRY);
        assert_eq!(frame.value(), 2047);
    }
}
