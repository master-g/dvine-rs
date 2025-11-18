//! ANM file format constants.
//!
//! This module contains all constant values used in the ANM (Animation) file format,
//! including file structure offsets, sizes, and special marker values.

/// Size of the file header (32 bytes)
pub const HEADER_SIZE: usize = 32;

/// Size of the index table (512 bytes = 256 entries × 2 bytes)
pub const INDEX_TABLE_SIZE: usize = 512;

/// Number of animation slots in the index table
pub const ANIMATION_SLOT_COUNT: usize = 256;

/// Offset where the index table starts in the file
pub const INDEX_TABLE_OFFSET: usize = 0x20;

/// Offset where animation data starts in the file
pub const ANIMATION_DATA_OFFSET: usize = 0x220;

/// Size of each frame descriptor in bytes (4 bytes)
pub const FRAME_DESCRIPTOR_SIZE: usize = 4;

/// End marker value (0xFFFF) - terminates animation sequence
pub const END_MARKER: u16 = 0xFFFF;

/// Jump marker value (0xFFFE) - changes `frame_index` to target
pub const JUMP_MARKER: u16 = 0xFFFE;

/// Sound marker value (0xFFFD) - triggers sound effect
pub const SOUND_MARKER: u16 = 0xFFFD;

/// Event marker value (0xFFFC) - marks game event
pub const EVENT_MARKER: u16 = 0xFFFC;

/// No animation marker (0xFFFF) - indicates empty slot in index table
pub const NO_ANIMATION: u16 = 0xFFFF;
