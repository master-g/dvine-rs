//! Animation sequence types for ANM files.
//!
//! This module defines the `AnimationSequence` struct which represents a complete
//! animation sequence consisting of multiple frame descriptors.

use crate::file::{DvFileError, FileType};

use super::{constants, frame::FrameDescriptor, parse_config::ParseConfig};

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
	/// Returns a tuple of:
	/// - The parsed `AnimationSequence`
	/// - The number of unique frame positions visited (for diagnostics)
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
	/// let (seq, bytes_read) = AnimationSequence::from_bytes(&data)?;
	/// assert_eq!(seq.len(), 2);
	/// # Ok::<(), Box<dyn std::error::Error>>(())
	/// ```
	///
	/// [`from_bytes_with_config`]: Self::from_bytes_with_config
	pub fn from_bytes(data: &[u8]) -> Result<(Self, usize), DvFileError> {
		Self::from_bytes_with_config(data, &ParseConfig::default())
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
	/// Returns a tuple of:
	/// - The parsed `AnimationSequence`
	/// - The number of unique frame positions visited
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
	/// let (seq, bytes_read) = AnimationSequence::from_bytes_with_config(&data, &config)?;
	/// # Ok::<(), Box<dyn std::error::Error>>(())
	/// ```
	pub fn from_bytes_with_config(
		data: &[u8],
		config: &ParseConfig,
	) -> Result<(Self, usize), DvFileError> {
		let mut frames = Vec::new();
		let mut frame_index: usize = 0;
		let mut iterations = 0;
		let mut visit_counts = std::collections::HashMap::new();
		let mut visited_positions = std::collections::HashSet::new();

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

			// Track visit count for this frame index
			let visits = visit_counts.entry(frame_index).or_insert(0);
			*visits += 1;
			if *visits > config.max_visits_per_index {
				// Loop detected - stop parsing but don't error
				// This is expected behavior for looping animations
				break;
			}

			// Calculate byte offset
			let offset = frame_index * constants::FRAME_DESCRIPTOR_SIZE;
			if offset + constants::FRAME_DESCRIPTOR_SIZE > data.len() {
				// Reached end of data
				break;
			}

			visited_positions.insert(frame_index);

			// Read frame descriptor
			let frame = FrameDescriptor::from_bytes(&data[offset..])?;

			match frame {
				FrameDescriptor::End => {
					frames.push(frame);
					break; // End marker - stop parsing
				}
				FrameDescriptor::Jump {
					target,
				} => {
					frames.push(frame);
					// Execute jump: set frame_index to target
					frame_index = target as usize;
					continue;
				}
				_ => {
					// Normal frame, Sound, or Event
					frames.push(frame);
					frame_index += 1; // Move to next frame
				}
			}
		}

		let bytes_read = visited_positions.len() * constants::FRAME_DESCRIPTOR_SIZE;
		Ok((
			Self {
				frames,
			},
			bytes_read,
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
