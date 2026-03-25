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

pub mod analysis;
#[cfg(feature = "binaural")]
pub mod binaural;
pub mod diffraction;
pub mod diffuse;
pub mod error;
pub mod image_source;
pub mod impulse;
pub mod integration;
pub mod material;
pub mod propagation;
pub mod ray;
pub mod resonance;
pub mod room;
#[cfg(feature = "wav")]
pub mod wav;

#[cfg(feature = "logging")]
pub mod logging;

pub use error::{GoonjError, Result};
pub use impulse::{ImpulseResponse, eyring_rt60, sabine_rt60};
pub use material::AcousticMaterial;
pub use propagation::{doppler_shift, inverse_square_law, speed_of_sound};
pub use ray::{AcousticRay, MultibandRay, RayHit, RayPath};
pub use resonance::{room_mode, schroeder_frequency};
pub use room::{AcceleratedRoom, AcousticRoom, RoomGeometry, Wall};
