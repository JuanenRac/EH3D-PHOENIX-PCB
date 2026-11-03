//! Typed errors of the model foundations. They carry data, not stable codes: the mapping to the
//! diagnostic codes of the suite is a separate step.

use thiserror::Error;

/// A value left the range of the design domain, or an operation could not be done exactly.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ModelError {
    /// A coordinate is not strictly inside `+-limit` nanometres.
    #[error("coordinate {value} nm is outside the design range of +-{limit} nm")]
    CoordinateOutOfRange { value: i128, limit: i64 },
    /// An angle is outside `+-limit` micro-degrees.
    #[error("angle {value} udeg is outside the range of +-{limit} udeg")]
    AngleOutOfRange { value: i128, limit: i64 },
    /// An intermediate result did not fit; the name says which operation.
    #[error("arithmetic overflow in {operation}")]
    Overflow { operation: &'static str },
    /// A decimal text could not be read.
    #[error("invalid decimal text: {reason}")]
    InvalidDecimal { reason: &'static str },
    /// An arc that has no defined shape.
    #[error("degenerate arc: {reason}")]
    DegenerateArc { reason: &'static str },
    /// A floating-point input was NaN or infinite.
    #[error("the number is not finite")]
    NotFinite,
}

/// Failures of the canonical JSON writer.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CanonicalError {
    /// An integer that a 64-bit float cannot hold exactly (JSON numbers are doubles in RFC 8785).
    #[error("the integer {0} is not exactly representable as a JSON number")]
    IntegerNotRepresentable(String),
    /// The value could not be turned into JSON.
    #[error("serialisation failed: {0}")]
    Serialise(String),
    /// NaN or infinity cannot be written.
    #[error("a number is not finite")]
    NotFinite,
    /// A value to quantise is too large to be exact.
    #[error("the number {0} is too large to quantise exactly")]
    QuantiseRange(String),
}

impl From<eh3d_container::CanonicalError> for CanonicalError {
    fn from(error: eh3d_container::CanonicalError) -> Self {
        match error {
            eh3d_container::CanonicalError::IntegerNotRepresentable(n) => {
                CanonicalError::IntegerNotRepresentable(n)
            }
            eh3d_container::CanonicalError::Serialise(m) => CanonicalError::Serialise(m),
            eh3d_container::CanonicalError::NotFinite => CanonicalError::NotFinite,
        }
    }
}
