use core::ops::Deref;

use super::{DshotError, Telemetry};

/// `DshotTelemetryFrame`: transmitted from the ESC to the Flight Controller (FC).
///
/// If Bidirectional `Dshot` is enabled and the Flight Controller sends a `DshotCommandFrame`
/// with the telemetry request bit set, the FC shifts its signal pin to an input right after
/// transmission finishes. The ESC then responds with a `DshotTelemetryFrame`.
///
/// This frame can be interpreted either as an `eRPM` (Electronic RPM) value, or an `EDT`
/// (Extended Dshot Telemetry) sensor payload.
///
/// ## `eRPM` Interpretation
/// When parsed as an `eRPM` frame, the 16-bit word uses the layout:
///
/// ```text
/// eeem mmmm mmmm cccc
/// ```
/// * `e`: 3-bit exponent
/// * `m`: 9-bit mantissa
/// * `c`: 4-bit inverted XOR checksum
///
/// The 9-bit mantissa value (M) is shifted left by the exponent (E) to calculate the core
/// commutation period in microseconds (M << E). This yields a period range of 1 µs to
/// 65,408 µs, translating to a minimum e-frequency of 15.29 Hz (for a standard 14-pole motor
/// with 7 pole-pairs, this represents a mechanical rotation frequency of 2.18 Hz).
///
/// ### `EDT` Interpretation
/// When parsed as an `EDT` frame, the 16-bit word uses the layout:
///
/// ```text
/// ttt0 dddd dddd cccc
/// ```
/// * `t`: 3-bit data type identifier (ie, 1 for Temperature, 2 for Voltage, etc)
/// * `0`: Static zero bit (forces the 4-bit prefix to evaluate as an even number)
/// * `d`: 8-bit sensor data payload
/// * `c`: 4-bit inverted XOR checksum
///
/// ## Differentiating Frames via the Prefix
/// The framework determines whether a packet represents an `eRPM` or `EDT` sequence by
/// inspecting the highest 4 bits of the 16-bit word (bits 12–15), known as the `prefix`:
///
/// ```text
/// pppp xxxx xxxx cccc
/// ```
/// This strategy capitalizes on GCR encoding redundancies, where a given commutation period
/// can mathematically be written in multiple ways. To prevent collisions, the ESC normalizes
/// `eRPM` values to guarantee an odd or zero prefix pattern.
///
/// * **It is an `eRPM` frame if:** The lower bit of the prefix is 1 (odd prefix), or the prefix evaluates to 0.
/// * **It is an `EDT` frame if:** The prefix evaluates to a non-zero, even number (meaning its lower bit is 0).
#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct DshotTelemetryFrame(u16);

impl Default for DshotTelemetryFrame {
    fn default() -> Self {
        Self::from_raw_12(0)
    }
}

impl TryFrom<u16> for DshotTelemetryFrame {
    type Error = DshotError;

    #[inline]
    fn try_from(raw_16: u16) -> Result<Self, DshotError> {
        Self::try_from_raw_16(raw_16)
    }
}

impl From<DshotTelemetryFrame> for u16 {
    #[inline]
    fn from(frame: DshotTelemetryFrame) -> Self {
        frame.raw_16()
    }
}

impl Deref for DshotTelemetryFrame {
    type Target = u16;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DshotTelemetryFrame {
    // Layout bitmasks matching the 16-bit word format: [eee mmmmmmmmm cccc]
    const CHECKSUM_BITS: u16 = 0x000F;
    const MANTISSA_BITS: u16 = 0x1FF0; // Bits 4 through 12
    const EXPONENT_BITS: u16 = 0xE000; // Bits 13 through 15 (Top 3 bits are Exponent)
    const ONE_MINUTE_IN_MICROSECONDS: u32 = 60_000_000;
    const ONE_MINUTE_IN_MICROSECONDS_F32: f32 = 60_000_000.0;

    #[inline]
    #[must_use]
    pub const fn from_raw_12(raw_12: u16) -> Self {
        Self((raw_12 << 4) | Self::calculate_checksum(raw_12))
    }

    /// # Errors
    pub fn try_from_raw_16(raw_16: u16) -> Result<Self, DshotError> {
        let ret = Self(raw_16);
        if ret.checksum_is_ok() {
            Ok(ret)
        } else {
            Err(DshotError::InvalidChecksum)
        }
    }

    #[inline]
    #[must_use]
    pub const fn raw_16(self) -> u16 {
        self.0
    }

    #[must_use]
    pub const fn calculate_checksum(raw_12: u16) -> u16 {
        (!(raw_12 ^ (raw_12 >> 4) ^ (raw_12 >> 8))) & 0x0F
    }

    #[inline]
    #[must_use]
    pub const fn checksum(self) -> u16 {
        self.0 & Self::CHECKSUM_BITS
    }

    /// Check if checksum is ok (XOR of all 4 nibbles must equal 0x0F).
    #[inline]
    #[must_use]
    pub const fn checksum_is_ok(self) -> bool {
        let checksum = (self.0 ^ (self.0 >> 4) ^ (self.0 >> 8) ^ (self.0 >> 12)) & 0x0F;
        checksum == 0x0F
    }

    #[inline]
    #[must_use]
    pub const fn from_exponent_mantissa(exponent: u16, mantissa: u16) -> Self {
        // Exponent is shifted up past the 9-bit mantissa block
        let raw_12 = ((exponent & 0x07) << 9) | (mantissa & 0x01FF);
        Self::from_raw_12(raw_12)
    }

    #[inline]
    #[must_use]
    pub fn from_type_value(data_type: u8, value: u8) -> Self {
        let raw_12 = (u16::from(data_type & 0x07) << 9) | u16::from(value);
        Self::from_raw_12(raw_12)
    }

    #[inline]
    #[must_use]
    pub const fn mantissa(self) -> u16 {
        (self.0 & Self::MANTISSA_BITS) >> 4
    }

    #[inline]
    #[must_use]
    pub const fn exponent(self) -> u16 {
        (self.0 & Self::EXPONENT_BITS) >> 13
    }

    #[inline]
    #[must_use]
    fn period_us(self) -> u32 {
        u32::from(self.mantissa()) << self.exponent()
    }

    #[inline]
    #[must_use]
    pub fn erpm(self) -> u32 {
        let raw_12 = (self.0 >> 4) & 0x0FFF;
        // Edge cases: if raw payload is 0 or maxed out, motor is stopped or invalid
        if raw_12 == 0 || raw_12 == 0x0FFF {
            return 0;
        }

        let period = self.period_us();
        if period == 0 {
            return 0;
        }
        Self::ONE_MINUTE_IN_MICROSECONDS / period
    }

    #[inline]
    #[must_use]
    pub fn erpm_f32(self) -> f32 {
        let raw_12 = (self.0 >> 4) & 0x0FFF;
        // Edge cases: if raw payload is 0 or maxed out, motor is stopped or invalid
        if raw_12 == 0 || raw_12 == 0x0FFF {
            return 0.0;
        }

        let period = self.period_us();
        if period == 0 {
            return 0.0;
        }
        #[allow(clippy::cast_precision_loss)]
        {
            Self::ONE_MINUTE_IN_MICROSECONDS_F32 / (period as f32)
        }
    }

    /// Helper method to safely identify whether the frame contains `eRPM` or `EDT` data.
    #[inline]
    #[must_use]
    pub const fn is_erpm_frame(self) -> bool {
        let raw_12 = (self.0 >> 4) & 0x0FFF;
        let prefix = (raw_12 >> 8) & 0x0F;
        (prefix == 0) || ((prefix & 0x01) != 0)
    }

    /// # Errors
    #[inline]
    pub fn try_decode_erpm(self) -> Result<u32, DshotError> {
        if self.is_erpm_frame() {
            Ok(self.erpm())
        } else {
            Err(DshotError::InvalidErpm)
        }
    }

    /// # Errors
    #[inline]
    pub fn try_decode_telemetry(self) -> Result<Telemetry, DshotError> {
        if self.is_erpm_frame() {
            return Ok(Telemetry::Erpm(self.erpm()));
        }

        // `EDT` Escape Mode Processing
        let raw_12 = (self.0 >> 4) & 0x0FFF;
        let prefix = (raw_12 >> 8) & 0x0F;

        let data_type = prefix >> 1;
        let data = (raw_12 & 0xFF) as u8;

        match data_type {
            1 => Ok(Telemetry::Temperature(data)),
            2 => Ok(Telemetry::Voltage(u32::from(data) * 250)),
            3 => Ok(Telemetry::Current(u32::from(data) * 1000)),
            4 => Ok(Telemetry::Debug1(data)),
            5 => Ok(Telemetry::Debug2(data)),
            6 => Ok(Telemetry::Debug3(data)),
            7 => Ok(Telemetry::StateEvent(data)),
            _ => Err(DshotError::InvalidTelemetry),
        }
    }
}

#[cfg(test)]
mod test_traits {
    use super::*;

    fn is_full<T: Sized + Send + Sync + Unpin + Copy + Clone + Default + PartialEq>() {}

    #[test]
    fn normal_types() {
        is_full::<DshotTelemetryFrame>();
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn test_default_constructor() {
        let frame = DshotTelemetryFrame::default();
        assert_eq!(frame.raw_16(), 0x000F); // raw_12 = 0, checksum = 0x0F
        assert!(frame.checksum_is_ok());
        assert_eq!(frame.erpm(), 0);
    }

    #[test]
    fn test_from_exponent_mantissa_packing() {
        // Create an `eRPM` frame with:
        // exponent = 2, mantissa = 0x5A (90 decimal)
        // raw_12 = (2 << 9) | 90 = 1024 | 90 = 1114 = 0x45A
        // expected checksum: !(0x4 ^ 0x5 ^ 0xA) & 0x0F = !0xB & 0x0F = 0x4
        // raw_16 = (0x45A << 4) | 0x4 = 0x45A4
        let frame = DshotTelemetryFrame::from_exponent_mantissa(2, 0x5A);

        assert_eq!(frame.raw_16(), 0x45A4);
        assert!(frame.checksum_is_ok());
        assert_eq!(frame.exponent(), 2);
        assert_eq!(frame.mantissa(), 0x5A);
    }

    #[test]
    fn test_erpm_calculation_active_motor() {
        // Using exponent = 1, mantissa = 300
        // (Note: Mantissa must be >= 256 so its MSB sets the prefix to an odd number for `EDT` compatibility)
        // raw_12 = (1 << 9) | 300 = 512 + 300 = 812 = 0x32C
        // expected checksum: !(0x3 ^ 0x2 ^ 0xC) & 0x0F = !0xD & 0x0F = 0x2
        // raw_16 = (0x32C << 4) | 0x2 = 0x32C2
        let frame = DshotTelemetryFrame::from_exponent_mantissa(1, 300);

        assert_eq!(frame.raw_16(), 0x32C2);
        assert!(frame.checksum_is_ok());
        assert!(frame.is_erpm_frame(), "Expected 0x32C to resolve as a valid eRPM prefix");

        // period_us = 300 << 1 = 600 microseconds
        // `eRPM` = 60_000_000 / 600 = 100_000 `eRPM`
        assert_eq!(frame.erpm(), 100_000);

        let decode_res = frame.try_decode_erpm();
        assert_eq!(decode_res.unwrap(), 100_000);
    }

    #[test]
    fn test_erpm_edge_cases_zero_and_max() {
        // Case 1: Zero payload (stopped motor)
        let zero_frame = DshotTelemetryFrame::from_raw_12(0);
        assert_eq!(zero_frame.erpm(), 0);
        assert_eq!(zero_frame.try_decode_erpm().unwrap(), 0);

        // Case 2: Maximum payload 0x0FFF (often indicates timeout or bad value)
        let max_frame = DshotTelemetryFrame::from_raw_12(0x0FFF);
        assert_eq!(max_frame.erpm(), 0);
        assert_eq!(max_frame.try_decode_erpm().unwrap(), 0);
    }

    #[test]
    fn test_edt_temperature_decoding() {
        // data_type = 1 (Temperature)
        // value = 85 (representing 85 degrees Celsius)
        // raw_12 = (1 << 9) | 85 = 512 | 85 = 597 = 0x255
        let frame = DshotTelemetryFrame::from_type_value(1, 85);

        assert!(!frame.is_erpm_frame());
        assert!(frame.try_decode_erpm().is_err());

        let telemetry = frame.try_decode_telemetry().unwrap();
        assert_eq!(telemetry, Telemetry::Temperature(85));
    }

    #[test]
    fn temperature() {
        let frame = DshotTelemetryFrame::from_type_value(1, 25);
        assert_eq!(frame.try_decode_telemetry(), Ok(Telemetry::Temperature(25)));
        let frame = DshotTelemetryFrame::from_type_value(1, 100);
        assert_eq!(frame.try_decode_telemetry(), Ok(Telemetry::Temperature(100)));
        let frame = DshotTelemetryFrame::from_type_value(1, 255);
        assert_eq!(frame.try_decode_telemetry(), Ok(Telemetry::Temperature(255)));
    }
    #[test]
    fn test_edt_voltage_decoding() {
        // data_type = 2 (Voltage)
        // value = 64 (representing 64 * 250mV = 16,000mV = 16.0V)
        // raw_12 = (2 << 9) | 64 = 1024 | 64 = 1088 = 0x440
        let frame = DshotTelemetryFrame::from_type_value(2, 64);

        let telemetry = frame.try_decode_telemetry().unwrap();
        assert_eq!(telemetry, Telemetry::Voltage(16_000));
    }

    #[test]
    fn test_edt_current_decoding() {
        // data_type = 3 (Current)
        // value = 25 (representing 25 * 1000mA = 25,000mA = 25A)
        // raw_12 = (3 << 9) | 25 = 1536 | 25 = 1561 = 0x619
        let frame = DshotTelemetryFrame::from_type_value(3, 25);

        let telemetry = frame.try_decode_telemetry().unwrap();
        assert_eq!(telemetry, Telemetry::Current(25_000));
    }

    #[test]
    fn test_edt_debug_and_state_events() {
        // Test data_type 4 (Debug1)
        let d1_frame = DshotTelemetryFrame::from_type_value(4, 42);
        assert_eq!(d1_frame.try_decode_telemetry().unwrap(), Telemetry::Debug1(42));

        // Test data_type 7 (StateEvent)
        let se_frame = DshotTelemetryFrame::from_type_value(7, 3);
        assert_eq!(se_frame.try_decode_telemetry().unwrap(), Telemetry::StateEvent(3));
    }

    #[test]
    fn test_invalid_checksum_rejection() {
        // Take a valid frame structure (0x45A4) and corrupt the lowest nibble checksum bits
        let corrupted_raw = 0x45A0;
        let frame_res = DshotTelemetryFrame::try_from_raw_16(corrupted_raw);

        assert!(frame_res.is_err(), "Expected constructor to reject invalid checksum structures");
    }
}
