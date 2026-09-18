use super::DshotError;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TelemetryType {
    #[default]
    Erpm = 0,
    Temperature = 1,
    Voltage = 2,
    Current = 3,
    Debug1 = 4,
    Debug2 = 5,
    StressLevel = 6,
    StateEvents = 7,
}

impl TryFrom<u8> for TelemetryType {
    type Error = DshotError;

    /// Validating conversion from `u8` to `TelemetryType`. Invalid values return error.
    fn try_from(value: u8) -> Result<Self, DshotError> {
        let default = Self::default();
        if value == default as u8 {
            Ok(default)
        } else {
            let ret = Self::from_u8(value);
            if ret == default {
                Err(DshotError::InvalidTelemetryType)
            } else {
                Ok(ret)
            }
        }
    }
}

impl TelemetryType {
    /// Forgiving conversion, converts invalid values to default.
    #[must_use]
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Erpm,
            1 => Self::Temperature,
            2 => Self::Voltage,
            3 => Self::Current,
            5 => Self::Debug1,
            6 => Self::Debug2,
            7 => Self::StressLevel,
            8 => Self::StateEvents,
            _ => Self::default(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Telemetry {
    Erpm(u32),
    /// 1°C per unit.
    Temperature(u8),
    /// 250mV per unit.
    Voltage(u32),
    /// 1A (1000mA) per unit.
    Current(u32),
    Debug1(u8),
    Debug2(u8),
    Debug3(u8),
    StateEvent(u8),
}

impl Default for Telemetry {
    fn default() -> Self {
        Self::Erpm(0)
    }
}

#[cfg(test)]
mod test_traits {
    use super::*;

    fn is_full_eq<T: Sized + Send + Sync + Unpin + Copy + Clone + Default + Eq + PartialEq>() {}

    #[test]
    fn normal_types() {
        is_full_eq::<TelemetryType>();
        is_full_eq::<Telemetry>();
    }
}
