//! Frame descriptor types for ANM animation sequences.
//!
//! This module defines the `FrameDescriptor` enum which represents individual
//! frame descriptors in an animation sequence. Each descriptor can be a regular
//! animation frame or a special control marker.

use crate::file::{DvFileError, FileType};

use super::constants;

/// Frame descriptor representing a single element in an animation sequence.
///
/// Each frame descriptor is 4 bytes (u16 id + u16 parameter) and can be one of:
/// - **Frame**: Regular animation frame with sprite ID and duration
/// - **End**: Marks the end of the animation sequence (0xFFFF)
/// - **Jump**: Changes the `frame_index` to target, enabling loops (0xFFFE)
/// - **Sound**: Triggers a sound effect (0xFFFD)
/// - **Event**: Marks a special game event (0xFFFC)
///
/// # State Machine Behavior
///
/// The parser simulates the original game engine's animation player:
/// - `Frame`: Display sprite, increment `frame_index`
/// - `Jump`: Set `frame_index` = target (enables loops)
/// - `End`: Stop parsing
/// - `Sound`/`Event`: Trigger action, increment `frame_index`
///
/// # Examples
///
/// ```
/// use dvine_types::file::anm::FrameDescriptor;
///
/// // Create a regular animation frame
/// let frame = FrameDescriptor::frame(10, 20);
/// assert!(frame.is_frame());
///
/// // Create a jump instruction (for looping)
/// let jump = FrameDescriptor::jump(0);
/// assert!(jump.is_jump());
///
/// // Create an end marker
/// let end = FrameDescriptor::end();
/// assert!(end.is_end());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameDescriptor {
	/// Regular animation frame
	Frame {
		/// Sprite frame ID to display
		frame_id: u16,
		/// Duration in ticks/milliseconds
		duration: u16,
	},

	/// End marker - terminates the animation sequence
	End,

	/// Jump instruction - sets `frame_index` to target (enables loops)
	Jump {
		/// Target frame index to jump to
		target: u16,
	},

	/// Sound effect trigger
	Sound {
		/// Sound effect ID to play
		sound_id: u16,
	},

	/// Event marker
	Event {
		/// Event ID
		event_id: u16,
	},
}

impl FrameDescriptor {
	/// Creates a new regular animation frame.
	///
	/// # Arguments
	/// * `frame_id` - Sprite frame ID to display
	/// * `duration` - Duration in ticks/milliseconds
	pub fn frame(frame_id: u16, duration: u16) -> Self {
		Self::Frame {
			frame_id,
			duration,
		}
	}

	/// Creates a frame using split duration components.
	///
	/// The low byte encodes the display time (ticks) and the high byte stores the
	/// auxiliary parameter that the original engine uses for scaling, hit windows,
	/// and other special behaviors.
	pub fn frame_with_duration_components(frame_id: u16, ticks: u8, parameter: u8) -> Self {
		let duration = u16::from(ticks) | (u16::from(parameter) << 8);
		Self::Frame {
			frame_id,
			duration,
		}
	}

	/// Creates an end marker that terminates the animation sequence.
	pub fn end() -> Self {
		Self::End
	}

	/// Creates a jump instruction that changes the `frame_index`.
	///
	/// # Arguments
	/// * `target` - Frame index to jump to
	///
	/// # Examples
	/// ```
	/// use dvine_types::file::anm::FrameDescriptor;
	///
	/// // Jump back to frame 0 (creates a loop)
	/// let jump = FrameDescriptor::jump(0);
	/// ```
	pub fn jump(target: u16) -> Self {
		Self::Jump {
			target,
		}
	}

	/// Creates a sound effect trigger.
	///
	/// # Arguments
	/// * `sound_id` - Sound effect ID to play
	pub fn sound(sound_id: u16) -> Self {
		Self::Sound {
			sound_id,
		}
	}

	/// Creates an event marker.
	///
	/// # Arguments
	/// * `event_id` - Event ID
	pub fn event(event_id: u16) -> Self {
		Self::Event {
			event_id,
		}
	}

	/// Returns `true` if this is an end marker.
	pub fn is_end(&self) -> bool {
		matches!(self, Self::End)
	}

	/// Returns `true` if this is a jump instruction.
	pub fn is_jump(&self) -> bool {
		matches!(self, Self::Jump { .. })
	}

	/// Returns `true` if this is a sound trigger.
	pub fn is_sound(&self) -> bool {
		matches!(self, Self::Sound { .. })
	}

	/// Returns `true` if this is an event marker.
	pub fn is_event(&self) -> bool {
		matches!(self, Self::Event { .. })
	}

	/// Returns `true` if this is a regular frame.
	pub fn is_frame(&self) -> bool {
		matches!(self, Self::Frame { .. })
	}

	/// Returns the low/high-byte duration components for regular frames.
	pub fn duration_components(&self) -> Option<(u8, u8)> {
		match self {
			Self::Frame {
				duration,
				..
			} => {
				let ticks = (*duration & 0x00FF) as u8;
				let parameter = (*duration >> 8) as u8;
				Some((ticks, parameter))
			}
			_ => None,
		}
	}

	/// Returns only the tick count (low byte) for regular frames.
	pub fn duration_ticks(&self) -> Option<u8> {
		self.duration_components().map(|(ticks, _)| ticks)
	}

	/// Returns the high-byte parameter for regular frames.
	pub fn duration_parameter(&self) -> Option<u8> {
		self.duration_components().map(|(_, param)| param)
	}

	/// Parses a frame descriptor from 4 bytes.
	///
	/// # Arguments
	/// * `data` - Byte slice containing at least 4 bytes
	///
	/// # Returns
	/// The parsed frame descriptor
	///
	/// # Errors
	/// Returns an error if the data is too short
	pub fn from_bytes(data: &[u8]) -> Result<Self, DvFileError> {
		if data.len() < constants::FRAME_DESCRIPTOR_SIZE {
			return Err(DvFileError::insufficient_data(
				FileType::Anm,
				constants::FRAME_DESCRIPTOR_SIZE,
				data.len(),
			));
		}

		let frame_id = u16::from_le_bytes([data[0], data[1]]);
		let param = u16::from_le_bytes([data[2], data[3]]);

		let descriptor = match frame_id {
			constants::END_MARKER => Self::End,
			constants::JUMP_MARKER => Self::Jump {
				target: param,
			},
			constants::SOUND_MARKER => Self::Sound {
				sound_id: param,
			},
			constants::EVENT_MARKER => Self::Event {
				event_id: param,
			},
			_ => Self::Frame {
				frame_id,
				duration: param,
			},
		};

		Ok(descriptor)
	}

	/// Converts the frame descriptor to 4 bytes.
	///
	/// # Returns
	/// A 4-byte array containing the serialized frame descriptor
	pub fn to_bytes(&self) -> [u8; constants::FRAME_DESCRIPTOR_SIZE] {
		let (frame_id, param) = match self {
			Self::Frame {
				frame_id,
				duration,
			} => (*frame_id, *duration),
			Self::End => (constants::END_MARKER, 0),
			Self::Jump {
				target,
			} => (constants::JUMP_MARKER, *target),
			Self::Sound {
				sound_id,
			} => (constants::SOUND_MARKER, *sound_id),
			Self::Event {
				event_id,
			} => (constants::EVENT_MARKER, *event_id),
		};

		let mut bytes = [0u8; constants::FRAME_DESCRIPTOR_SIZE];
		bytes[0..2].copy_from_slice(&frame_id.to_le_bytes());
		bytes[2..4].copy_from_slice(&param.to_le_bytes());
		bytes
	}
}

impl std::fmt::Display for FrameDescriptor {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Frame {
				frame_id,
				duration,
			} => write!(f, "Frame(id={}, dur={})", frame_id, duration),
			Self::End => write!(f, "End"),
			Self::Jump {
				target,
			} => write!(f, "Jump(→{})", target),
			Self::Sound {
				sound_id,
			} => write!(f, "Sound({})", sound_id),
			Self::Event {
				event_id,
			} => write!(f, "Event({})", event_id),
		}
	}
}
