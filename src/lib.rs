//! # Goonj
//!
//! **Goonj** (गूँज — Hindi/Urdu for "echo, resonance") — acoustics engine for
//! the AGNOS ecosystem.
//!
//! Provides sound propagation, room simulation, ray-based acoustic tracing,
//! impulse response generation, diffraction, and resonance analysis.
//! Built on [`hisab`] for math.
//!
//! ## Example
//!
//! ```rust
//! use goonj::{propagation, impulse, material::AcousticMaterial};
//!
//! let speed = propagation::speed_of_sound(20.0);
//! assert!((speed - 343.4).abs() < 0.1);
//!
//! let rt60 = impulse::sabine_rt60(500.0, 50.0);
//! assert!(rt60 > 0.0);
//! ```

pub mod error;
pub mod material;
pub mod propagation;
pub mod room;
pub mod impulse;
pub mod ray;
pub mod diffraction;
pub mod resonance;

#[cfg(feature = "logging")]
pub mod logging;

pub use error::{GoonjError, Result};
pub use material::AcousticMaterial;
pub use propagation::{speed_of_sound, inverse_square_law, doppler_shift};
pub use impulse::{ImpulseResponse, sabine_rt60, eyring_rt60};
pub use room::{Wall, RoomGeometry, AcousticRoom};
pub use ray::{AcousticRay, RayHit};
pub use resonance::{room_mode, schroeder_frequency};
