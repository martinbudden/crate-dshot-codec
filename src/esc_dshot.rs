use super::DshotSpeed;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EscDshot {
    protocol: DshotSpeed,
    cpu_frequency: u32,
    data_high_pulse_width: u16,
    data_low_pulse_width: u16,
    wrap_cycle_count: u16,
}

impl Default for EscDshot {
    fn default() -> Self {
        Self::new(DshotSpeed::Dshot150)
    }
}

impl EscDshot {
    pub const DEFAULT_CPU_FREQUENCY: u32 = 150_000_000;
    const DSHOT_BIT_COUNT: usize = 16;
    const DMA_BUFFER_SIZE: usize = Self::DSHOT_BIT_COUNT + 1;

    const DSHOT150_T0H: u32 = 2500;
    const DSHOT150_T1H: u32 = 5000;
    const DSHOT150_T: u32 = 6680;

    #[must_use]
    pub const fn new(protocol: DshotSpeed) -> Self {
        let mut this = Self {
            protocol,
            cpu_frequency: Self::DEFAULT_CPU_FREQUENCY,

            data_high_pulse_width: 0,
            data_low_pulse_width: 0,
            wrap_cycle_count: 0,
        };
        this.set_protocol(protocol);
        this
    }

    /// Set the `cpu_frequency` of a newly constructed `EscDshot`.
    #[must_use]
    pub const fn with_cpu_frequency(mut self, cpu_frequency: u32) -> Self {
        self.set_cpu_frequency(cpu_frequency);
        self
    }
}

#[allow(unused)]
impl EscDshot {
    #[must_use]
    pub const fn nano_seconds_to_cycles(self, nano_seconds: u32) -> u16 {
        // note: the k values cancel out, but give greater precision in the calculation
        const K: u64 = 128;
        let d = K * 1_000_000_000 / (self.cpu_frequency as u64);
        #[allow(clippy::cast_possible_truncation)]
        {
            ((nano_seconds as u64) * K / d) as u16
        }
    }

    pub const fn set_cpu_frequency(&mut self, cpu_frequency: u32) {
        self.cpu_frequency = cpu_frequency;
        self.set_protocol(self.protocol);
    }

    pub const fn set_protocol(&mut self, protocol: DshotSpeed) {
        self.protocol = protocol;

        // data_low_pulse_width and data_high_pulse_width are in processor cycles
        // for RPI_PICO: default CPU frequency is 150MHz, that is 0.15GHz
        self.data_low_pulse_width = self.nano_seconds_to_cycles(Self::DSHOT150_T0H); // =  375 = 2500 * 0.15GHz
        self.data_high_pulse_width = self.nano_seconds_to_cycles(Self::DSHOT150_T1H); // =  750 = 5000 * 0.15GHz
        self.wrap_cycle_count = self.nano_seconds_to_cycles(Self::DSHOT150_T); // = 1002 = 6680 * 0.15GHz

        match protocol {
            DshotSpeed::Dshot150 => {
                _ = protocol;
            }
            DshotSpeed::Dshot300 => {
                self.data_low_pulse_width /= 2;
                self.data_high_pulse_width /= 2;
                self.wrap_cycle_count /= 2;
            }
            DshotSpeed::Dshot600 => {
                self.data_low_pulse_width /= 4;
                self.data_high_pulse_width /= 4;
                self.wrap_cycle_count /= 4;
            }
            DshotSpeed::Dshot1200 => {
                self.data_low_pulse_width /= 8;
                self.data_high_pulse_width /= 8;
                self.wrap_cycle_count /= 8;
            }
        }
    }

    /// Returns an array of duty cycles for use in PWM DMA.
    ///
    /// The array an extra element set to zero to ensure that PWM output gets pulled low at the end of the sequence.
    #[must_use]
    pub fn duty_cycles_u16(&self, frame: u16) -> [u16; Self::DMA_BUFFER_SIZE] {
        let mut ret = [0u16; Self::DMA_BUFFER_SIZE];

        let mut mask_bit = 1 << (Self::DSHOT_BIT_COUNT - 1);
        for item in &mut ret {
            *item = if frame & mask_bit == 0 { self.data_high_pulse_width } else { self.data_low_pulse_width };
            mask_bit >>= 1;
        }

        // Set last value to zero, (DMA_BUFFER_SIZE = DSHOT_BIT_COUNT + 1).
        ret[Self::DMA_BUFFER_SIZE - 1] = 0;
        ret
    }

    /// Returns an array of duty cycles for use in PWM DMA.
    ///
    /// The array an extra element set to zero to ensure that PWM output gets pulled low at the end of the sequence.
    #[must_use]
    pub fn duty_cycles_u32(&self, frame: u16, use_high_order_bits: bool) -> [u32; Self::DMA_BUFFER_SIZE] {
        let mut ret = [0u32; Self::DMA_BUFFER_SIZE];

        let mut mask_bit = 1 << (Self::DSHOT_BIT_COUNT - 1);
        if use_high_order_bits {
            for item in &mut ret {
                let byte = if frame & mask_bit == 0 {
                    u32::from(self.data_high_pulse_width)
                } else {
                    u32::from(self.data_low_pulse_width)
                };
                *item = byte << 16;
                mask_bit >>= 1;
            }
        } else {
            for item in &mut ret {
                *item = if frame & mask_bit == 0 {
                    u32::from(self.data_high_pulse_width)
                } else {
                    u32::from(self.data_low_pulse_width)
                };
                mask_bit >>= 1;
            }
        }

        // Set last value to zero, (DMA_BUFFER_SIZE = DSHOT_BIT_COUNT + 1).
        ret[Self::DMA_BUFFER_SIZE - 1] = 0;
        ret
    }
}

#[cfg(test)]
mod test_traits {
    use super::*;

    fn is_full<T: Sized + Send + Sync + Unpin + Copy + Clone + Default + PartialEq>() {}

    #[test]
    fn normal_types() {
        is_full::<EscDshot>();
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn nano_seconds_to_cycles() {
        let esc = EscDshot::new(DshotSpeed::Dshot150);
        assert_eq!(126, esc.nano_seconds_to_cycles(840));
        assert_eq!(187, esc.nano_seconds_to_cycles(1250));
        assert_eq!(313, esc.nano_seconds_to_cycles(2090));
        assert_eq!(375, esc.nano_seconds_to_cycles(2500));
        assert_eq!(750, esc.nano_seconds_to_cycles(5000));
        assert_eq!(1125, esc.nano_seconds_to_cycles(7500));

        assert_eq!(350, esc.nano_seconds_to_cycles(2333));
        assert_eq!(700, esc.nano_seconds_to_cycles(4666));
        assert_eq!(1000, esc.nano_seconds_to_cycles(6666));
    }
    #[test]
    fn pulse_widths() {
        let esc150 = EscDshot::new(DshotSpeed::Dshot150);
        assert_eq!(750, esc150.data_high_pulse_width);
        assert_eq!(375, esc150.data_low_pulse_width);

        let esc300 = EscDshot::new(DshotSpeed::Dshot300);
        assert_eq!(375, esc300.data_high_pulse_width);
        assert_eq!(187, esc300.data_low_pulse_width);

        let esc600 = EscDshot::new(DshotSpeed::Dshot600);
        assert_eq!(187, esc600.data_high_pulse_width);
        assert_eq!(93, esc600.data_low_pulse_width);
    }
}
