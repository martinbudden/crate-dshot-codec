use super::DshotError;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum DshotSpeed {
    #[default]
    Dshot150 = 0,
    Dshot300 = 1,
    Dshot600 = 2,
    Dshot1200 = 3,
}

impl DshotSpeed {
    #[must_use]
    pub const fn baud_rate(self) -> u32 {
        match self {
            Self::Dshot150 => 150_000,
            Self::Dshot300 => 300_000,
            Self::Dshot600 => 600_000,
            Self::Dshot1200 => 1_200_000,
        }
    }
}

impl TryFrom<u8> for DshotSpeed {
    type Error = DshotError;

    /// Validating conversion from `u8` to `Protocol`. Invalid values return error.
    fn try_from(value: u8) -> Result<Self, DshotError> {
        let default = Self::default();
        if value == default as u8 {
            Ok(default)
        } else {
            let ret = Self::from_u8(value);
            if ret == default {
                Err(DshotError::InvalidDshotSpeed)
            } else {
                Ok(ret)
            }
        }
    }
}

impl DshotSpeed {
    /// Forgiving conversion from `u8` to `Protocol`, converts invalid values to default.
    #[must_use]
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Dshot300,
            2 => Self::Dshot600,
            3 => Self::Dshot1200,
            _ => Self::Dshot150,
        }
    }
}

#[cfg(test)]
mod test_traits {
    use super::*;

    fn is_full_eq<T: Sized + Send + Sync + Unpin + Copy + Clone + Default + Eq + PartialEq>() {}

    #[test]
    fn normal_types() {
        is_full_eq::<DshotSpeed>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_u8() {
        // from_u8 is a forgiving conversion.
        assert_eq!(DshotSpeed::from_u8(0), DshotSpeed::Dshot150);
        assert_eq!(DshotSpeed::from_u8(1), DshotSpeed::Dshot300);
        assert_eq!(DshotSpeed::from_u8(2), DshotSpeed::Dshot600);
        assert_eq!(DshotSpeed::from_u8(3), DshotSpeed::Dshot1200);
        assert_eq!(DshotSpeed::from_u8(4), DshotSpeed::Dshot150);
        assert_eq!(DshotSpeed::from_u8(255), DshotSpeed::Dshot150);
    }
    #[test]
    fn try_from_() {
        // With try_from invalid values give error.
        assert_eq!(DshotSpeed::try_from(0), Ok(DshotSpeed::Dshot150));
        assert_eq!(DshotSpeed::try_from(1), Ok(DshotSpeed::Dshot300));
        assert_eq!(DshotSpeed::try_from(2), Ok(DshotSpeed::Dshot600));
        assert_eq!(DshotSpeed::try_from(3), Ok(DshotSpeed::Dshot1200));
        assert_eq!(DshotSpeed::try_from(4), Err(DshotError::InvalidDshotSpeed));
        assert_eq!(DshotSpeed::try_from(255), Err(DshotError::InvalidDshotSpeed));
    }
    #[test]
    fn baud_rates() {
        assert_eq!(DshotSpeed::Dshot150.baud_rate(), 150_000);
        assert_eq!(DshotSpeed::Dshot300.baud_rate(), 300_000);
        assert_eq!(DshotSpeed::Dshot600.baud_rate(), 600_000);
        assert_eq!(DshotSpeed::Dshot1200.baud_rate(), 1_200_000);
    }
}
