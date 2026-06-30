//! Integration APIs for downstream consumers.
//!
//! Feature-gated modules providing consumer-specific types and convenience APIs:
//! - **dhvani** (`dhvani-compat`): IR handoff for convolution reverb
//! - **kiran** (`kiran-compat`): Real-time occlusion queries for game audio
//! - **soorat** (`soorat-compat`): Visualization data structures

#[cfg(feature = "dhvani-compat")]
pub mod dhvani;

#[cfg(feature = "kiran-compat")]
pub mod kiran;

#[cfg(feature = "soorat-compat")]
pub mod soorat;
