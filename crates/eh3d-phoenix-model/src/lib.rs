//! Foundations of the design model.
//!
//! Four small, independent modules:
//!
//! * [`units`] and [`geometry`]: lengths in integer nanometres inside a checked range, angles in
//!   integer micro-degrees, orientation predicates that fit in `i64` by construction and arcs
//!   stored as end points plus a signed sweep.
//! * [`id`]: object identities as UUID version 7, produced by a generator whose clock and random
//!   source are injected, so tests are reproducible. Writers receive ids; they never create them.
//! * [`canonical`] and [`quantise`]: canonical JSON as in RFC 8785 plus a final line feed, and the
//!   quantisation of millimetre and degree values before they are written.
//! * [`writer_profile`]: the name of the writer profile that every deterministic output carries, as one
//!   comment with one rule (what a name may hold, how it is written for Gerber, Excellon and XML, how it is
//!   read back).
//!
//! There is no project file here: only the pieces whose rules are fixed.

pub mod canonical;
pub mod diagnostic;
pub mod error;
pub mod geometry;
pub mod paths;
pub mod quantise;
pub mod units;
pub mod writer_profile;

pub use eh3d_container::{
    Clock, DeterministicRandom, IdError, IdGenerator, ObjectId, RandomSource, StepClock,
    SystemClock, SystemRandom,
};
pub use error::{CanonicalError, ModelError};
pub use geometry::{orient, segments_intersect, Arc, Orientation, Point, Rect};
pub use paths::{is_reserved_device_name, output_path_problem};
pub use units::{Nm, ScaledDecimal, Udeg, ANGLE_LIMIT_UDEG, COORD_LIMIT_NM};
