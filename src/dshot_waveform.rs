use super::{DshotCommandFrame, DshotSpeed};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DshotMotorMasks {
    pub motors: [u32; DshotWaveform::MOTOR_COUNT],
}

impl DshotMotorMasks {
    #[must_use]
    pub const fn new(m1: u8, m2: u8, m3: u8, m4: u8) -> Self {
        Self { motors: [1u32 << m1, 1u32 << m2, 1u32 << m3, 1u32 << m4] }
    }

    #[must_use]
    pub const fn all(&self) -> u32 {
        self.motors[0] | self.motors[1] | self.motors[2] | self.motors[3]
    }
}

/// Timing parameters for a `Dshot` waveform.
///
/// The waveform uses `ticks_per_bit` timer/DMA events for each Dshot bit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DshotTiming {
    /// `Dshot` baud rate, eg 600,000 for `Dshot600`.
    pub baud_rate: u32,
    /// Number of DMA/timer ticks per `Dshot` bit.
    pub ticks_per_bit: u8,
    /// Number of ticks for a zero bit to remain high.
    pub zero_high_ticks: u8,
    /// Number of ticks for a one bit to remain high.
    pub one_high_ticks: u8,
}

impl Default for DshotTiming {
    fn default() -> Self {
        Self::new(DshotSpeed::Dshot300)
    }
}

impl DshotTiming {
    #[must_use]
    pub const fn new(dshot_speed: DshotSpeed) -> Self {
        let baud_rate = dshot_speed.baud_rate();

        let this = Self { baud_rate, ticks_per_bit: 8, zero_high_ticks: 3, one_high_ticks: 6 };
        debug_assert!(this.ticks_per_bit as usize <= DshotWaveform::BUFFER_LEN / DshotWaveform::BITS_PER_PACKET);
        debug_assert!(this.zero_high_ticks <= this.ticks_per_bit);
        debug_assert!(this.one_high_ticks <= this.ticks_per_bit);

        this
    }

    #[must_use]
    pub const fn tick_rate(self) -> u32 {
        self.baud_rate * self.ticks_per_bit as u32
    }
}

/// A complete four-motor GPIO waveform.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DshotWaveform {
    data: [u32; Self::BUFFER_LEN],
}

impl Default for DshotWaveform {
    fn default() -> Self {
        Self::new()
    }
}

impl DshotWaveform {
    pub const MOTOR_COUNT: usize = 4;

    pub const BITS_PER_PACKET: usize = 16;
    pub const TICKS_PER_BIT: usize = 8;
    pub const BUFFER_LEN: usize = Self::BITS_PER_PACKET * Self::TICKS_PER_BIT;

    #[must_use]
    pub const fn new() -> Self {
        Self { data: [0; Self::BUFFER_LEN] }
    }
}

impl DshotWaveform {
    #[must_use]
    pub fn as_slice(&self) -> &[u32] {
        &self.data
    }

    pub fn encode_frames(
        &mut self,
        frames: [DshotCommandFrame; Self::MOTOR_COUNT],
        masks: DshotMotorMasks,
        timing: DshotTiming,
    ) {
        let packets = [frames[0].raw(), frames[1].raw(), frames[2].raw(), frames[3].raw()];
        self.encode(packets, masks, timing);
    }

    pub fn encode(&mut self, packets: [u16; Self::MOTOR_COUNT], masks: DshotMotorMasks, timing: DshotTiming) {
        self.data.fill(0);

        let all = masks.all();

        for bit in 0..Self::BITS_PER_PACKET {
            let shift = 15 - bit;

            let mut zero = 0u32;
            let mut one = 0u32;

            for (motor_idx, &mask) in masks.motors.iter().enumerate() {
                if packets[motor_idx] & (1 << shift) != 0 {
                    one |= mask;
                } else {
                    zero |= mask;
                }
            }

            let base = bit * Self::TICKS_PER_BIT;

            // Every motor starts its symbol high.
            self.data[base] = all;

            // Zero bits fall earlier.
            if timing.zero_high_ticks < timing.ticks_per_bit {
                self.data[base + usize::from(timing.zero_high_ticks)] = zero << 16;
            }

            // One bits fall later.
            if timing.one_high_ticks < timing.ticks_per_bit {
                self.data[base + usize::from(timing.one_high_ticks)] = one << 16;
            }
        }
    }
}

#[cfg(test)]
mod test_traits {
    use super::*;

    fn is_full_eq<T: Sized + Send + Sync + Unpin + Copy + Clone + Default>() {}

    #[test]
    fn normal_types() {
        is_full_eq::<DshotMotorMasks>();
        is_full_eq::<DshotTiming>();
        is_full_eq::<DshotWaveform>();
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    const M1: u32 = 1 << 6; // PB6
    const M2: u32 = 1 << 7; // PB7
    const M3: u32 = 1 << 0; // PB0
    const M4: u32 = 1 << 1; // PB1

    const MOTORS: DshotMotorMasks = DshotMotorMasks { motors: [M1, M2, M3, M4] };

    const ALL: u32 = M1 | M2 | M3 | M4;

    #[test]
    fn buffer_is_zero_initially() {
        let waveform = DshotWaveform::new();

        assert!(waveform.as_slice().iter().all(|&x| x == 0));
    }

    #[test]
    fn all_zero_packet() {
        let mut waveform = DshotWaveform::new();
        let timing = DshotTiming::new(DshotSpeed::Dshot600);

        waveform.encode([0x0000, 0x0000, 0x0000, 0x0000], MOTORS, timing);

        let data = waveform.as_slice();

        // Every bit starts with all four motors high.
        for bit in 0..16 {
            let base = bit * 8;

            assert_eq!(data[base], ALL);

            // Zero bits fall after 3 ticks.
            assert_eq!(data[base + 3], ALL << 16);

            // There are no one bits to reset at tick 6.
            assert_eq!(data[base + 6], 0);
        }
    }

    #[test]
    fn all_one_packet() {
        let mut waveform = DshotWaveform::new();
        let timing = DshotTiming::new(DshotSpeed::Dshot600);

        waveform.encode([0xffff, 0xffff, 0xffff, 0xffff], MOTORS, timing);

        let data = waveform.as_slice();

        for bit in 0..16 {
            let base = bit * 8;

            // Every bit starts high.
            assert_eq!(data[base], ALL);

            // There are no zero bits.
            assert_eq!(data[base + 3], 0);

            // One bits fall after 6 ticks.
            assert_eq!(data[base + 6], ALL << 16);
        }
    }

    #[test]
    fn alternating_bits() {
        let mut waveform = DshotWaveform::new();
        let timing = DshotTiming::new(DshotSpeed::Dshot600);

        // MSB first:
        //
        // 1010101010101010
        //
        // Every motor has the same packet.
        let packet = 0xaaaa;

        waveform.encode([packet, packet, packet, packet], MOTORS, timing);

        let data = waveform.as_slice();

        for bit in 0..16 {
            let base = bit * 8;

            assert_eq!(data[base], ALL);

            if bit % 2 == 0 {
                // 1 bit.
                assert_eq!(data[base + 3], 0);
                assert_eq!(data[base + 6], ALL << 16);
            } else {
                // 0 bit.
                assert_eq!(data[base + 3], ALL << 16);
                assert_eq!(data[base + 6], 0);
            }
        }
    }

    #[test]
    fn motors_are_independent() {
        let mut waveform = DshotWaveform::new();
        let timing = DshotTiming::new(DshotSpeed::Dshot600);

        waveform.encode(
            [
                0x8000, // M1: 1
                0x0000, // M2: 0
                0x8000, // M3: 1
                0x0000, // M4: 0
            ],
            MOTORS,
            timing,
        );

        let data = waveform.as_slice();

        // First bit:
        //
        // M1 = 1
        // M2 = 0
        // M3 = 1
        // M4 = 0
        //
        // All go high initially.
        assert_eq!(data[0], ALL);

        // The zero motors fall at tick 3.
        assert_eq!(data[3], (M2 | M4) << 16);

        // The one motors fall at tick 6.
        assert_eq!(data[6], (M1 | M3) << 16);
    }

    #[test]
    fn each_motor_can_have_different_data() {
        let mut waveform = DshotWaveform::new();
        let timing = DshotTiming::new(DshotSpeed::Dshot600);

        waveform.encode([0x8000, 0x4000, 0x2000, 0x1000], MOTORS, timing);

        let data = waveform.as_slice();

        // Bit 15:
        //
        // M1 = 1
        // M2 = 0
        // M3 = 0
        // M4 = 0
        assert_eq!(data[0], ALL);
        assert_eq!(data[3], (M2 | M3 | M4) << 16);
        assert_eq!(data[6], M1 << 16);

        // Bit 14:
        //
        // M1 = 0
        // M2 = 1
        // M3 = 0
        // M4 = 0
        let base = 8;

        assert_eq!(data[base], ALL);
        assert_eq!(data[base + 3], (M1 | M3 | M4) << 16);
        assert_eq!(data[base + 6], M2 << 16);

        // Bit 13:
        //
        // M1 = 0
        // M2 = 0
        // M3 = 1
        // M4 = 0
        let base = 16;

        assert_eq!(data[base], ALL);
        assert_eq!(data[base + 3], (M1 | M2 | M4) << 16);
        assert_eq!(data[base + 6], M3 << 16);

        // Bit 12:
        //
        // M1 = 0
        // M2 = 0
        // M3 = 0
        // M4 = 1
        let base = 24;

        assert_eq!(data[base], ALL);
        assert_eq!(data[base + 3], (M1 | M2 | M3) << 16);
        assert_eq!(data[base + 6], M4 << 16);
    }

    #[test]
    fn correct_buffer_length() {
        let mut waveform = DshotWaveform::new();
        let timing = DshotTiming::new(DshotSpeed::Dshot600);

        waveform.encode([0x1234, 0x5678, 0x9abc, 0xdef0], MOTORS, timing);

        assert_eq!(waveform.as_slice().len(), 128);
    }

    #[test]
    fn only_expected_bits_are_set() {
        let mut waveform = DshotWaveform::new();
        let timing = DshotTiming::new(DshotSpeed::Dshot600);

        waveform.encode([0x1234, 0x5678, 0x9abc, 0xdef0], MOTORS, timing);

        let allowed = ALL | (ALL << 16);

        for &value in waveform.as_slice() {
            assert_eq!(value & !allowed, 0, "unexpected GPIO bits set: {value:#010x}");
        }
    }

    #[test]
    fn timing_constants_are_correct() {
        assert_eq!(DshotTiming::new(DshotSpeed::Dshot150).tick_rate(), 1_200_000);
        assert_eq!(DshotTiming::new(DshotSpeed::Dshot300).tick_rate(), 2_400_000);
        assert_eq!(DshotTiming::new(DshotSpeed::Dshot600).tick_rate(), 4_800_000);
        assert_eq!(DshotTiming::new(DshotSpeed::Dshot1200).tick_rate(), 9_600_000);
    }

    #[test]
    fn waveform_has_128_dma_words() {
        assert_eq!(DshotWaveform::BUFFER_LEN, 16 * 8);

        let waveform = DshotWaveform::default();
        assert_eq!(waveform.as_slice().len(), 128);
    }
}
