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

/// Ambisonics encoding (1st-order B-Format and 3rd-order HOA).
pub mod ambisonics;
/// Room acoustics analysis metrics (C50, C80, D50, EDT, G, ts, LF, IACC, STI).
pub mod analysis;
/// Beam tracing — volumetric sound propagation without sampling artifacts.
pub mod beam;
/// Binaural impulse response generation with HRTF spatialization.
#[cfg(feature = "binaural")]
pub mod binaural;
/// Cross-crate bridges — primitive-value conversions from other AGNOS science crates.
pub mod bridge;
/// Coupled room acoustics — multi-room energy exchange and double-slope decay.
pub mod coupled;
/// Edge diffraction loss and occlusion detection (UTD, BTM).
pub mod diffraction;
/// Stochastic ray tracing (diffuse rain) for late reverberation tails.
pub mod diffuse;
/// Acoustic diffusion equation — energy density PDE solver.
pub mod diffusion;
/// Source directivity patterns (omnidirectional, cardioid, tabulated balloon data).
pub mod directivity;
/// Error types for the goonj crate.
pub mod error;
/// Feedback Delay Network (FDN) for efficient late reverberation synthesis.
pub mod fdn;
/// Hybrid frequency crossover — blending wave-based and geometric results.
pub mod hybrid;
/// Image-source method for exact early specular reflections.
pub mod image_source;
/// Impulse response generation, RT60 estimation, and energy decay curves.
pub mod impulse;
/// Integration APIs for downstream consumers (dhvani, kiran, soorat).
pub mod integration;
/// Acoustic materials with frequency-dependent absorption, scattering, and transmission.
pub mod material;
/// ISO 9613-2 outdoor sound propagation methods.
pub mod outdoor;
/// Portal-based sound propagation through openings between rooms.
pub mod portal;
/// Sound propagation: speed of sound, inverse square law, Doppler, atmospheric effects.
pub mod propagation;
/// Acoustic radiosity — energy exchange between surface patches.
pub mod radiosity;
/// Acoustic ray tracing: single-band, multiband, BVH-accelerated.
pub mod ray;
/// Room resonance modes, Schroeder frequency, and modal density.
pub mod resonance;
/// Room geometry, walls, and acceleration structures.
pub mod room;
/// Vector-based scattering for diffuse reflections (cosine-weighted hemisphere sampling).
pub mod scattering;
/// Universal Diffraction Filter Approximation (UDFA) for real-time diffraction.
pub mod udfa;
/// Underwater acoustics — ocean sound speed, seabed reflection, absorption.
#[cfg(feature = "underwater")]
pub mod underwater;
/// Vibroacoustics — structural vibration to acoustic radiation coupling.
pub mod vibroacoustics;
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
