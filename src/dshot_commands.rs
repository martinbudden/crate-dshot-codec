use super::DshotError;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum DshotCommand {
    #[default]
    MotorStop = 0,
    /// Wait at least 260ms before next command.
    Beep1,
    /// Wait at least 260ms before next command.
    Beep2,
    /// Wait at least 260ms before next command.
    Beep3,
    /// Wait at least 260ms before next command.
    Beep4,
    /// Wait at least 260ms before next command.
    Beep5,
    /// Wait at least 12ms before next command.
    EscInfo,
    /// Needs 6 transmissions.
    SpinDirection1,
    /// Needs 6 transmissions.
    SpinDirection2,
    /// Needs 6 transmissions.
    ThreeDModeOn,
    /// Needs 6 transmissions.
    ThreeDModeOff,
    SettingsRequest,
    /// Needs 6 transmissions. Wait at least 35ms before next command.
    SettingsSave,
    /// Needs 6 transmissions.
    ExtendedTelemetryEnable,
    /// Needs 6 transmissions.
    ExtendedTelemetryDisable,

    // 15-19 are unassigned.
    /// Needs 6 transmissions.
    SpinDirectionNormal = 20,
    /// Needs 6 transmissions.
    SpinDirectionReversed,
    Led0On,
    Led1On,
    Led2On,
    Led3On,
    Led0Off,
    Led1Off,
    Led2Off,
    Led3Off,
    AudioStreamModeToggle,
    SilentModeToggle,
    /// Needs 6 transmissions. Enables individual signal line commands.
    SignalLineTelemetryEnable,
    /// Needs 6 transmissions. Disables individual signal line commands.
    SignalLineTelemetryDisable,
    /// Needs 6 transmissions. Enables individual signal line commands.
    SignalLineContinuousErpmTelemetry,
    /// Needs 6 transmissions. Enables individual signal line commands.
    SignalLineContinuousErpmPeriodTelemetry,

    // 36-41 are unassigned.
    /// Temperature: 1ºC per LSB.
    SignalLineTemperatureTelemetry = 42,
    /// Voltage: 10mV per LSB, 40.95V max.
    SignalLineVoltageTelemetry,
    /// Current: 100mA per LSB, 409.5A max.
    SignalLineCurrentTelemetry,
    /// Power: 10mAh per LSB, 40.95Ah max.
    SignalLineConsumptionTelemetry,
    /// erpm: 100erpm per LSB, 409500erpm max.
    SignalLineErpmTelemetry,
    /// 16us per LSB, 65520us max.
    SignalLineErpmPeriodTelemetry,
}

impl TryFrom<u8> for DshotCommand {
    type Error = DshotError;

    /// Validating conversion from `u8` to `Command`. Invalid values return error.
    fn try_from(value: u8) -> Result<Self, DshotError> {
        let default = Self::default();
        if value == default as u8 {
            Ok(default)
        } else {
            let ret = Self::from_u8(value);
            if ret == default {
                Err(DshotError::InvalidDshotCommand)
            } else {
                Ok(ret)
            }
        }
    }
}

impl DshotCommand {
    /// Forgiving conversion from `u8` to `Command`, converts invalid values to default.
    #[must_use]
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::MotorStop,
            1 => Self::Beep1,
            2 => Self::Beep2,
            3 => Self::Beep3,
            4 => Self::Beep4,
            5 => Self::Beep5,
            6 => Self::EscInfo,
            7 => Self::SpinDirection1,
            8 => Self::SpinDirection2,
            9 => Self::ThreeDModeOn,
            10 => Self::ThreeDModeOff,
            11 => Self::SettingsRequest,
            12 => Self::SettingsSave,
            13 => Self::ExtendedTelemetryEnable,
            14 => Self::ExtendedTelemetryDisable,
            20 => Self::SpinDirectionNormal,
            21 => Self::SpinDirectionReversed,
            22 => Self::Led0On,
            23 => Self::Led1On,
            24 => Self::Led2On,
            25 => Self::Led3On,
            26 => Self::Led0Off,
            27 => Self::Led1Off,
            28 => Self::Led2Off,
            29 => Self::Led3Off,
            30 => Self::AudioStreamModeToggle,
            31 => Self::SilentModeToggle,
            32 => Self::SignalLineTelemetryEnable,
            33 => Self::SignalLineTelemetryDisable,
            34 => Self::SignalLineContinuousErpmTelemetry,
            35 => Self::SignalLineContinuousErpmPeriodTelemetry,
            // 36-41 are unassigned.
            42 => Self::SignalLineTemperatureTelemetry,
            43 => Self::SignalLineVoltageTelemetry,
            44 => Self::SignalLineCurrentTelemetry,
            45 => Self::SignalLineConsumptionTelemetry,
            46 => Self::SignalLineErpmTelemetry,
            47 => Self::SignalLineErpmPeriodTelemetry,
            _ => Self::default(),
        }
    }
}

impl DshotCommand {
    #[must_use]
    pub const fn repetitions_required(self) -> u8 {
        match self {
            // Protocol documentation often states 6 repeats for these commands.
            // We use 10, like Betaflight, as a conservative measure.
            Self::SpinDirection1
            | Self::SpinDirection2
            | Self::ThreeDModeOn
            | Self::ThreeDModeOff
            | Self::SettingsSave
            | Self::ExtendedTelemetryEnable
            | Self::ExtendedTelemetryDisable
            | Self::SpinDirectionNormal
            | Self::SpinDirectionReversed
            | Self::SignalLineTelemetryEnable
            | Self::SignalLineTelemetryDisable
            | Self::SignalLineContinuousErpmTelemetry
            | Self::SignalLineContinuousErpmPeriodTelemetry => 10,
            _ => 1,
        }
    }

    #[allow(unused)]
    #[must_use]
    pub const fn delay_required_us(self) -> u32 {
        match self {
            Self::Beep1 | Self::Beep2 | Self::Beep3 | Self::Beep4 | Self::Beep5 => 260_000,
            Self::EscInfo => 12_000,
            Self::SettingsSave => 35_000,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod test_traits {
    use super::*;

    fn is_full_eq<T: Sized + Send + Sync + Unpin + Copy + Clone + Default + Eq + PartialEq>() {}

    #[test]
    fn normal_types() {
        is_full_eq::<DshotCommand>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u8_to_command() {
        // Test forgiving conversions.
        assert_eq!(DshotCommand::from_u8(0), DshotCommand::MotorStop);
        assert_eq!(DshotCommand::from_u8(1), DshotCommand::Beep1);
        assert_eq!(DshotCommand::from_u8(35), DshotCommand::SignalLineContinuousErpmPeriodTelemetry);
        assert_eq!(DshotCommand::from_u8(36), DshotCommand::MotorStop);
        assert_eq!(DshotCommand::from_u8(37), DshotCommand::MotorStop);
        assert_eq!(DshotCommand::from_u8(38), DshotCommand::MotorStop);
        assert_eq!(DshotCommand::from_u8(39), DshotCommand::MotorStop);
        assert_eq!(DshotCommand::from_u8(40), DshotCommand::MotorStop);
        assert_eq!(DshotCommand::from_u8(41), DshotCommand::MotorStop);
        assert_eq!(DshotCommand::from_u8(42), DshotCommand::SignalLineTemperatureTelemetry);
        assert_eq!(DshotCommand::from_u8(47), DshotCommand::SignalLineErpmPeriodTelemetry);
        assert_eq!(DshotCommand::from_u8(48), DshotCommand::MotorStop);
        assert_eq!(DshotCommand::from_u8(49), DshotCommand::MotorStop);
        assert_eq!(DshotCommand::from_u8(50), DshotCommand::MotorStop);
        assert_eq!(DshotCommand::from_u8(255), DshotCommand::MotorStop);

        // Test unforgiving conversions.
        assert_eq!(DshotCommand::try_from(0), Ok(DshotCommand::MotorStop));
        assert_eq!(DshotCommand::try_from(1), Ok(DshotCommand::Beep1));
        assert_eq!(DshotCommand::try_from(35), Ok(DshotCommand::SignalLineContinuousErpmPeriodTelemetry));
        assert_eq!(DshotCommand::try_from(36), Err(DshotError::InvalidDshotCommand));
        assert_eq!(DshotCommand::try_from(37), Err(DshotError::InvalidDshotCommand));
        assert_eq!(DshotCommand::try_from(38), Err(DshotError::InvalidDshotCommand));
        assert_eq!(DshotCommand::try_from(39), Err(DshotError::InvalidDshotCommand));
        assert_eq!(DshotCommand::try_from(40), Err(DshotError::InvalidDshotCommand));
        assert_eq!(DshotCommand::try_from(41), Err(DshotError::InvalidDshotCommand));
        assert_eq!(DshotCommand::try_from(42), Ok(DshotCommand::SignalLineTemperatureTelemetry));
        assert_eq!(DshotCommand::try_from(47), Ok(DshotCommand::SignalLineErpmPeriodTelemetry));
        assert_eq!(DshotCommand::try_from(48), Err(DshotError::InvalidDshotCommand));
        assert_eq!(DshotCommand::try_from(49), Err(DshotError::InvalidDshotCommand));
        assert_eq!(DshotCommand::try_from(50), Err(DshotError::InvalidDshotCommand));
        assert_eq!(DshotCommand::try_from(255), Err(DshotError::InvalidDshotCommand));
    }
}
