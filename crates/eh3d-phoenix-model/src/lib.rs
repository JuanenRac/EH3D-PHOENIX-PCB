//! Foundations of the design model.
//!
//! Three small, independent modules:
//!
//! * [`units`] and [`geometry`]: lengths in integer nanometres inside a checked range, angles in
//!   integer micro-degrees, orientation predicates that fit in `i64` by construction and arcs
//!   stored as end points plus a signed sweep.
//! * [`id`]: object identities as UUID version 7, produced by a generator whose clock and random
//!   source are injected, so tests are reproducible. Writers receive ids; they never create them.
//! * [`canonical`] and [`quantise`]: canonical JSON as in RFC 8785 plus a final line feed, and the
//!   quantisation of millimetre and degree values before they are written.
//!
//! There is no project file here: only the pieces whose rules are fixed.

pub mod canonical;
pub mod diagnostic;
pub mod error;
pub mod geometry;
pub mod quantise;
pub mod units;

pub use eh3d_container::{
    Clock, DeterministicRandom, IdError, IdGenerator, ObjectId, RandomSource, StepClock,
    SystemClock, SystemRandom,
};
pub use error::{CanonicalError, ModelError};
pub use geometry::{orient, segments_intersect, Arc, Orientation, Point, Rect};
pub use units::{Nm, ScaledDecimal, Udeg, ANGLE_LIMIT_UDEG, COORD_LIMIT_NM};
