#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DshotError {
    NotImplemented,
    TxTimeout,
    /// ESC did not respond to telemetry request in time.
    RxTimeout,
    NoGcrData,
    InvalidDshotCommand,
    InvalidDshotSpeed,
    InvalidTelemetry,
    InvalidTelemetryType,
    InvalidErpm,
    InvalidRunLength,
    InvalidNrziData,
    InvalidChecksum,
}

#[cfg(test)]
mod test_traits {
    use super::*;

    fn is_full_eq_no_default<T: Sized + Send + Sync + Unpin + Copy + Clone + Eq + PartialEq>() {}

    #[test]
    fn normal_types() {
        is_full_eq_no_default::<DshotError>();
    }
}
