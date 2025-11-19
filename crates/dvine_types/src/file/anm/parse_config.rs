//! Parse configuration for ANM animation sequence parsing.
//!
//! This module provides configuration options for controlling loop detection
//! and parsing limits when reading ANM animation sequences.

/// Configuration for parsing animation sequences.
///
/// Controls loop detection and parsing limits to prevent infinite loops
/// and excessive memory consumption when parsing malformed or complex ANM files.
///
/// # Loop Detection
///
/// The parser simulates the original game engine's animation player state machine,
/// which means it executes jump instructions and can encounter loops. The configuration
/// provides two levels of protection:
///
/// 1. **`max_iterations`**: Total number of parsing iterations before stopping
/// 2. **`max_visits_per_index`**: How many times a single `frame_index` can be visited
///
/// # Presets
///
/// Three presets are available for common use cases:
/// - `default()`: Balanced limits (5000 iterations, 128 visits/index)
/// - `lenient()`: Higher limits for complex looping animations (10000, 512)
/// - `strict()`: Lower limits for simple files (1000, 32)
///
/// # Examples
///
/// ```
/// use dvine_types::file::anm::ParseConfig;
///
/// // Use default configuration
/// let config = ParseConfig::default();
///
/// // Use lenient mode for complex files with many loops
/// let config = ParseConfig::lenient();
///
/// // Create custom configuration
/// let config = ParseConfig::new(2000, 20);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseConfig {
	/// Maximum number of total iterations before stopping (prevents infinite loops)
	pub max_iterations: usize,
	/// Maximum number of times a single frame index can be visited
	pub max_visits_per_index: usize,
}

impl Default for ParseConfig {
	fn default() -> Self {
		Self {
			max_iterations: 5000,
			max_visits_per_index: 128,
		}
	}
}

impl ParseConfig {
	/// Create a new parse configuration with custom limits.
	///
	/// # Arguments
	/// * `max_iterations` - Total iteration limit
	/// * `max_visits_per_index` - Per-frame visit limit
	pub fn new(max_iterations: usize, max_visits_per_index: usize) -> Self {
		Self {
			max_iterations,
			max_visits_per_index,
		}
	}

	/// Create a lenient configuration with higher limits.
	///
	/// Suitable for complex animations with many loops or intricate jump patterns.
	/// - `max_iterations`: 10000
	/// - `max_visits_per_index`: 512
	pub fn lenient() -> Self {
		Self {
			max_iterations: 10000,
			max_visits_per_index: 512,
		}
	}

	/// Create a strict configuration with lower limits.
	///
	/// Suitable for simple animations or when you want faster parsing with
	/// early termination of complex loops.
	/// - `max_iterations`: 1000
	/// - `max_visits_per_index`: 32
	pub fn strict() -> Self {
		Self {
			max_iterations: 1000,
			max_visits_per_index: 32,
		}
	}
}
