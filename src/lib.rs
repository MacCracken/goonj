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

/// Room acoustics analysis metrics (C50, C80, D50, STI, absorption placement).
pub mod analysis;
/// Binaural impulse response generation with HRTF spatialization.
#[cfg(feature = "binaural")]
pub mod binaural;
/// Edge diffraction loss and occlusion detection.
pub mod diffraction;
/// Stochastic ray tracing (diffuse rain) for late reverberation tails.
pub mod diffuse;
/// Error types for the goonj crate.
pub mod error;
/// Image-source method for exact early specular reflections.
pub mod image_source;
/// Impulse response generation, RT60 estimation, and energy decay curves.
pub mod impulse;
/// Integration APIs for downstream consumers (dhvani, kiran, soorat).
pub mod integration;
/// Acoustic materials with frequency-dependent absorption and scattering.
pub mod material;
/// Sound propagation: speed of sound, inverse square law, Doppler, atmospheric effects.
pub mod propagation;
/// Acoustic ray tracing: single-band, multiband, BVH-accelerated.
pub mod ray;
/// Room resonance modes, Schroeder frequency, and modal density.
pub mod resonance;
/// Room geometry, walls, and acceleration structures.
pub mod room;
/// WAV file export (16-bit PCM, mono/stereo).
#[cfg(feature = "wav")]
pub mod wav;

/// Tracing subscriber initialization.
#[cfg(feature = "logging")]
pub mod logging;

pub use error::{GoonjError, Result};
pub use impulse::{ImpulseResponse, eyring_rt60, sabine_rt60};
pub use material::AcousticMaterial;
pub use propagation::{doppler_shift, inverse_square_law, speed_of_sound};
pub use ray::{AcousticRay, MultibandRay, RayHit, RayPath};
pub use resonance::{room_mode, schroeder_frequency};
pub use room::{AcceleratedRoom, AcousticRoom, RoomGeometry, Wall};
