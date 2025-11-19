//! Animation sequence types for ANM files.
//!
//! This module defines the `AnimationSequence` struct which represents a complete
//! animation sequence consisting of multiple frame descriptors.

use crate::file::{DvFileError, FileType};

use super::{constants, frame::FrameDescriptor, parse_config::ParseConfig};

/// Diagnostics produced while simulating an ANM sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SequenceParseStats {
	/// Total number of bytes that were visited/consumed while parsing.
	pub bytes_consumed: usize,
	/// Number of unique frame positions that were evaluated.
	pub unique_frame_positions: usize,
	/// Whether the parser encountered an explicit end marker (0xFFFF).
	pub terminated_by_end_marker: bool,
	/// Whether parsing stopped because the loop-detection guard tripped.
	pub loop_detected: bool,
}

impl SequenceParseStats {
	/// Returns true when parsing ended cleanly on an end marker with no loop detection.
	pub fn ended_cleanly(&self) -> bool {
		self.terminated_by_end_marker && !self.loop_detected
	}
}

/// Represents a complete animation sequence with a list of frame descriptors.
///
/// An animation sequence is a series of frame descriptors that define an animation.
/// It can contain regular frames, jump instructions (for loops), sound triggers,
/// event markers, and must typically end with an End marker.
///
/// # State Machine Parsing
///
/// The parser simulates the original game engine's animation player:
/// - Maintains a `frame_index` (current position in the animation)
/// - Starts at `frame_index` = 0
/// - Increments for normal frames
/// - Jumps to target for Jump (0xFFFE) instructions
/// - Stops at End (0xFFFF) marker
/// - Includes loop detection to prevent infinite loops
///
/// # Examples
///
/// ```
/// use dvine_types::file::anm::{AnimationSequence, FrameDescriptor};
///
/// // Create a simple animation sequence
/// let mut seq = AnimationSequence::new();
/// seq.add_frame(FrameDescriptor::frame(0, 10));
/// seq.add_frame(FrameDescriptor::frame(1, 10));
/// seq.add_end_marker();
///
/// assert_eq!(seq.len(), 3);
/// assert!(seq.has_end_marker());
/// ```
///
/// # Looping Animations
///
/// ```
/// use dvine_types::file::anm::{AnimationSequence, FrameDescriptor};
///
/// // Create a looping animation
/// let mut seq = AnimationSequence::new();
/// seq.add_frame(FrameDescriptor::frame(0, 10));
/// seq.add_frame(FrameDescriptor::frame(1, 10));
/// seq.add_frame(FrameDescriptor::jump(0)); // Jump back to start
/// seq.add_end_marker();
///
/// // When parsed, this will detect the loop and stop gracefully
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationSequence {
	frames: Vec<FrameDescriptor>,
}

impl AnimationSequence {
	/// Creates a new empty animation sequence.
	///
	/// # Examples
	///
	/// ```
	/// use dvine_types::file::anm::AnimationSequence;
	///
	/// let seq = AnimationSequence::new();
	/// assert!(seq.is_empty());
	/// ```
	pub fn new() -> Self {
		Self {
			frames: Vec::new(),
		}
	}

	/// Creates an animation sequence from a vector of frame descriptors.
	///
	/// # Arguments
	///
	/// * `frames` - Vector of frame descriptors
	///
	/// # Examples
	///
	/// ```
	/// use dvine_types::file::anm::{AnimationSequence, FrameDescriptor};
	///
	/// let frames = vec![
	///     FrameDescriptor::frame(0, 10),
	///     FrameDescriptor::frame(1, 10),
	///     FrameDescriptor::end(),
	/// ];
	/// let seq = AnimationSequence::from_frames(frames);
	/// assert_eq!(seq.len(), 3);
	/// ```
	pub fn from_frames(frames: Vec<FrameDescriptor>) -> Self {
		Self {
			frames,
		}
	}

	/// Returns a reference to the frame descriptors.
	pub fn frames(&self) -> &[FrameDescriptor] {
		&self.frames
	}

	/// Returns a mutable reference to the frame descriptors.
	pub fn frames_mut(&mut self) -> &mut Vec<FrameDescriptor> {
		&mut self.frames
	}

	/// Adds a frame descriptor to the sequence.
	///
	/// # Arguments
	///
	/// * `frame` - Frame descriptor to add
	///
	/// # Examples
	///
	/// ```
	/// use dvine_types::file::anm::{AnimationSequence, FrameDescriptor};
	///
	/// let mut seq = AnimationSequence::new();
	/// seq.add_frame(FrameDescriptor::frame(5, 20));
	/// assert_eq!(seq.len(), 1);
	/// ```
	pub fn add_frame(&mut self, frame: FrameDescriptor) {
		self.frames.push(frame);
	}

	/// Adds an end marker to the sequence.
	///
	/// # Examples
	///
	/// ```
	/// use dvine_types::file::anm::AnimationSequence;
	///
	/// let mut seq = AnimationSequence::new();
	/// seq.add_end_marker();
	/// assert!(seq.has_end_marker());
	/// ```
	pub fn add_end_marker(&mut self) {
		self.frames.push(FrameDescriptor::end());
	}

	/// Returns the number of frames in the sequence (including markers).
	pub fn len(&self) -> usize {
		self.frames.len()
	}

	/// Returns true if the sequence is empty.
	pub fn is_empty(&self) -> bool {
		self.frames.is_empty()
	}

	/// Returns true if the sequence ends with an end marker.
	pub fn has_end_marker(&self) -> bool {
		self.frames.last().is_some_and(FrameDescriptor::is_end)
	}

	/// Calculates the total byte size of this sequence when serialized.
	pub fn byte_size(&self) -> usize {
		self.frames.len() * constants::FRAME_DESCRIPTOR_SIZE
	}

	/// Converts the sequence to bytes.
	///
	/// # Returns
	///
	/// A byte vector containing all frame descriptors serialized sequentially.
	pub fn to_bytes(&self) -> Vec<u8> {
		let mut bytes = Vec::with_capacity(self.byte_size());
		for frame in &self.frames {
			bytes.extend_from_slice(&frame.to_bytes());
		}
		bytes
	}

	/// Parse an animation sequence from a byte slice using default configuration.
	///
	/// This is a convenience wrapper around [`from_bytes_with_config`] using
	/// default parsing limits (1000 iterations, 10 visits per index).
	///
	/// # Arguments
	///
	/// * `data` - Byte slice containing the animation data
	///
	/// # Returns
	///
	/// Returns a tuple containing the parsed sequence and detailed diagnostics
	/// gathered during parsing.
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - Maximum iterations exceeded (likely infinite loop)
	/// - Data is malformed or truncated
	///
	/// # Examples
	///
	/// ```no_run
	/// use dvine_types::file::anm::AnimationSequence;
	///
	/// let data = vec![
	///     0x01, 0x00, 0x0A, 0x00,  // Frame(id=1, duration=10)
	///     0xFF, 0xFF, 0x00, 0x00,  // End
	/// ];
	///
	/// let (seq, stats) = AnimationSequence::from_bytes(&data)?;
	/// assert_eq!(seq.len(), 2);
	/// assert!(stats.ended_cleanly());
	/// # Ok::<(), Box<dyn std::error::Error>>(())
	/// ```
	///
	/// [`from_bytes_with_config`]: Self::from_bytes_with_config
	pub fn from_bytes(data: &[u8]) -> Result<(Self, SequenceParseStats), DvFileError> {
		Self::from_bytes_with_config(data, &ParseConfig::default())
	}

	/// Parses an animation sequence from bytes in raw mode (without simulating jumps).
	///
	/// This method reads frame descriptors sequentially from the byte stream without
	/// executing jump instructions. It stops when it encounters an End marker or runs
	/// out of data. This is useful for editing tools that need to preserve the original
	/// structure of the animation data, including jump targets.
	///
	/// # Arguments
	///
	/// * `data` - Byte slice containing the animation sequence data
	///
	/// # Returns
	///
	/// A tuple containing:
	/// - The parsed animation sequence (with original structure preserved)
	/// - The number of bytes read
	///
	/// # Errors
	///
	/// Returns an error if the data is insufficient or malformed.
	///
	/// # Examples
	///
	/// ```
	/// use dvine_types::file::anm::{AnimationSequence, FrameDescriptor};
	///
	/// // Create animation data with a jump
	/// let mut data = Vec::new();
	/// data.extend_from_slice(&FrameDescriptor::frame(0, 10).to_bytes());
	/// data.extend_from_slice(&FrameDescriptor::jump(0).to_bytes());
	/// data.extend_from_slice(&FrameDescriptor::end().to_bytes());
	///
	/// let (seq, bytes_read) = AnimationSequence::from_bytes_raw(&data)?;
	/// // In raw mode, this returns exactly 3 frames (not expanded loop)
	/// assert_eq!(seq.len(), 3);
	/// assert!(seq.frames()[1].is_jump());
	/// # Ok::<(), Box<dyn std::error::Error>>(())
	/// ```
	pub fn from_bytes_raw(data: &[u8]) -> Result<(Self, usize), DvFileError> {
		let mut frames = Vec::new();
		let mut offset = 0;

		loop {
			if offset + constants::FRAME_DESCRIPTOR_SIZE > data.len() {
				return Err(DvFileError::insufficient_data(
					FileType::Anm,
					offset + constants::FRAME_DESCRIPTOR_SIZE,
					data.len(),
				));
			}

			let frame = FrameDescriptor::from_bytes(&data[offset..])?;
			let is_end = frame.is_end();
			frames.push(frame);
			offset += constants::FRAME_DESCRIPTOR_SIZE;

			// Stop at End marker
			if is_end {
				break;
			}
		}

		Ok((
			Self {
				frames,
			},
			offset,
		))
	}

	/// Parse an animation sequence from a byte slice with custom configuration.
	///
	/// This method simulates the original game's animation player state machine:
	/// - Starts with `frame_index = 0`
	/// - Reads frame descriptors at `data[frame_index * 4..]`
	/// - Increments `frame_index` for normal frames
	/// - Sets `frame_index` to the target for Jump (0xFFFE) instructions
	/// - Stops when encountering End (0xFFFF) marker
	/// - Includes loop detection to prevent infinite loops
	///
	/// This approach correctly handles:
	/// - Looping animations (Jump instructions pointing backwards)
	/// - Shared data regions
	/// - Complex animation patterns
	///
	/// # Arguments
	///
	/// * `data` - Byte slice containing the animation data
	/// * `config` - Parse configuration controlling loop detection limits
	///
	/// # Returns
	///
	/// Returns a tuple of the parsed `AnimationSequence` and diagnostics that capture
	/// how much data was consumed and whether loop detection was triggered.
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - Maximum iterations exceeded (likely infinite loop)
	/// - Data is malformed or truncated
	///
	/// # Examples
	///
	/// ```no_run
	/// use dvine_types::file::anm::{AnimationSequence, ParseConfig};
	///
	/// let data = vec![
	///     0x01, 0x00, 0x0A, 0x00,  // Frame(id=1, duration=10)
	///     0xFE, 0xFF, 0x00, 0x00,  // Jump(target=0) - loop!
	/// ];
	///
	/// // Use lenient config for complex loops
	/// let config = ParseConfig::lenient();
	/// let (seq, stats) = AnimationSequence::from_bytes_with_config(&data, &config)?;
	/// # Ok::<(), Box<dyn std::error::Error>>(())
	/// ```
	pub fn from_bytes_with_config(
		data: &[u8],
		config: &ParseConfig,
	) -> Result<(Self, SequenceParseStats), DvFileError> {
		let mut frames = Vec::new();
		let mut frame_index: usize = 0;
		let mut iterations = 0;
		let mut visit_counts = std::collections::HashMap::new();
		let mut visited_positions = std::collections::HashSet::new();
		let mut terminated_by_end_marker = false;
		let mut loop_detected = false;

		loop {
			iterations += 1;
			if iterations > config.max_iterations {
				return Err(DvFileError::EntryNotFound {
					file_type: FileType::Anm,
					message:
						"Maximum iterations exceeded - possible infinite loop in animation sequence"
							.to_string(),
				});
			}

			let visits = visit_counts.entry(frame_index).or_insert(0);
			*visits += 1;
			if *visits > config.max_visits_per_index {
				loop_detected = true;
				break;
			}

			let offset = frame_index * constants::FRAME_DESCRIPTOR_SIZE;
			if offset + constants::FRAME_DESCRIPTOR_SIZE > data.len() {
				return Err(DvFileError::insufficient_data(
					FileType::Anm,
					offset + constants::FRAME_DESCRIPTOR_SIZE,
					data.len(),
				));
			}

			visited_positions.insert(frame_index);

			let frame = FrameDescriptor::from_bytes(&data[offset..])?;

			match frame {
				FrameDescriptor::End => {
					terminated_by_end_marker = true;
					frames.push(frame);
					break;
				}
				FrameDescriptor::Jump {
					target,
				} => {
					frames.push(frame);
					frame_index = target as usize;
					continue;
				}
				_ => {
					frames.push(frame);
					frame_index += 1;
				}
			}
		}

		let stats = SequenceParseStats {
			bytes_consumed: visited_positions.len() * constants::FRAME_DESCRIPTOR_SIZE,
			unique_frame_positions: visited_positions.len(),
			terminated_by_end_marker,
			loop_detected,
		};

		Ok((
			Self {
				frames,
			},
			stats,
		))
	}
}

impl Default for AnimationSequence {
	fn default() -> Self {
		Self::new()
	}
}

impl std::fmt::Display for AnimationSequence {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "AnimationSequence({} frames)", self.frames.len())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::file::anm::{FrameDescriptor, ParseConfig, constants};

	fn serialize_frames(frames: &[FrameDescriptor]) -> Vec<u8> {
		frames.iter().flat_map(|frame| frame.to_bytes()).collect()
	}

	#[test]
	fn from_bytes_reports_clean_termination_stats() {
		let data = serialize_frames(&[
			FrameDescriptor::frame(1, 5),
			FrameDescriptor::frame_with_duration_components(2, 3, 1),
			FrameDescriptor::end(),
		]);

		let (sequence, stats) = AnimationSequence::from_bytes(&data).expect("sequence parses");
		assert_eq!(sequence.len(), 3);
		assert!(stats.ended_cleanly());
		assert_eq!(stats.unique_frame_positions, 3);
		assert_eq!(stats.bytes_consumed, 3 * constants::FRAME_DESCRIPTOR_SIZE);
	}

	#[test]
	fn from_bytes_with_config_flags_loop_detection() {
		let data = serialize_frames(&[FrameDescriptor::frame(1, 10), FrameDescriptor::jump(0)]);

		let config = ParseConfig::new(10, 1);
		let (_sequence, stats) =
			AnimationSequence::from_bytes_with_config(&data, &config).expect("sequence parses");

		assert!(stats.loop_detected, "loop guard should trigger");
		assert!(!stats.terminated_by_end_marker);
		assert_eq!(stats.unique_frame_positions, 2);
		assert_eq!(stats.bytes_consumed, 2 * constants::FRAME_DESCRIPTOR_SIZE);
		assert!(!stats.ended_cleanly());
	}

	#[test]
	fn from_bytes_raw_errors_when_frame_truncated() {
		let truncated = vec![0x01, 0x00];
		let err = AnimationSequence::from_bytes_raw(&truncated).expect_err("parse should fail");
		assert!(matches!(err, DvFileError::InsufficientData { .. }));
	}
}
