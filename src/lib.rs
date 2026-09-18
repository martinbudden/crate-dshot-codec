#![no_std]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
//#![deny(missing_docs)]
#![deny(
    missing_copy_implementations,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unused_must_use,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    unused_results
)]
#![warn(unused_results)]
#![warn(clippy::pedantic)]
#![warn(clippy::doc_paragraphs_missing_punctuation)]

mod dshot_command_frame;
mod dshot_commands;
mod dshot_errors;
mod dshot_speed;
mod dshot_telemetry;
mod dshot_telemetry_frame;
mod esc_dshot;
mod gcr_frame;
mod nrzi_frame;

pub use esc_dshot::EscDshot;

pub use dshot_command_frame::DshotCommandFrame;
pub use dshot_commands::DshotCommand;
pub use dshot_errors::DshotError;
pub use dshot_speed::DshotSpeed;
pub use dshot_telemetry::{Telemetry, TelemetryType};
pub use dshot_telemetry_frame::DshotTelemetryFrame;
pub use gcr_frame::GcrFrame;
pub use nrzi_frame::NrziFrame;
