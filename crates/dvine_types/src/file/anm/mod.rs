//! `.ANM` file format support for `dvine-rs` project.
//!
//! This module provides support for loading and manipulating ANM (Animation) files
//! used in the `D+VINE[LUV]` visual novel engine. ANM files contain animation sequences
//! that reference sprite frames with timing information.
//!
//! # File Structure Overview
//!
//! The ANM format is a binary format with a fixed structure consisting of:
//! - **Header (0x00-0x1F):** 32 bytes containing file identifier
//! - **Index Table (0x20-0x21F):** 512 bytes (256 u16 entries) mapping animation slots to data offsets
//! - **Animation Data (0x220+):** Variable-length animation sequences with frame descriptors
//!
//! ## Header Structure (32 bytes at offset 0x00)
//!
//! ```text
//! Offset  Size  Field         Description
//! ------  ----  ------------  ------------------------------------------
//! 0x00    32    identifier    File identifier (typically all zeros or magic bytes)
//! ```
//!
//! ## Index Table (512 bytes at offset 0x20)
//!
//! ```text
//! Location: 0x0020
//! Size:     512 bytes (256 entries × 2 bytes each)
//! Format:   Array of u16 (little-endian) offsets
//!
//! Each entry is a word offset (multiply by 2 to get byte offset) from the start of the
//! animation data region (0x220) to the beginning of an animation sequence.
//! An offset of 0xFFFF indicates no animation.
//! Note: Multiple slots may point to the same or overlapping data regions - this is legal
//! and used for space optimization or creating animation variants.
//! ```
//!
//! ### Index Calculation
//!
//! Given an animation slot index (0-255):
//! - File offset of index entry = `0x20 + (slot_index × 2)`
//! - Word offset value = Read u16 at that location
//! - Byte offset = `word_offset` × 2
//! - Actual file offset = `0x220 + byte_offset`
//!
//! ## Animation Data Region (starts at 0x220)
//!
//! ### Animation Sequence Structure
//!
//! Each animation sequence consists of:
//! 1. A series of frame descriptors (4 bytes each)
//! 2. An end marker (0xFFFF)
//! 3. Optional padding (0x0000)
//!
//! ### Frame Descriptor (4 bytes)
//!
//! ```text
//! Offset  Size  Field        Description
//! ------  ----  -----------  ------------------------------------------
//! +0x00   2     frame_id     Sprite frame ID (or special value)
//! +0x02   2     parameter    Frame duration, jump target, or other data
//! ```
//!
//! ### Special Frame ID Values
//!
//! - `0xFFFF`: End marker - terminates the animation sequence
//! - `0xFFFE`: Jump marker - changes `frame_index` to the target (enables loops)
//! - `0xFFFD`: Sound effect trigger
//! - `0xFFFC`: Event marker
//! - Other values: Sprite frame IDs to display
//!
//! ### State Machine Parsing
//!
//! The parser simulates the original game's animation player state machine:
//! - Maintains a `frame_index` (position in the animation data)
//! - Starts at `frame_index` = 0 for each animation slot
//! - Increments `frame_index` after processing normal frames
//! - When encountering Jump (0xFFFE), sets `frame_index` to the jump target
//! - When encountering End (0xFFFF), stops parsing
//! - Includes loop detection to prevent infinite loops
//!
//! This approach correctly handles:
//! - Looping animations (Jump instructions that point backwards)
//! - Shared data regions (multiple slots starting at different positions)
//! - Complex animation patterns
//!
//! ## Example File Analysis
//!
//! ### BGMAGIC.anm (Simple Example)
//!
//! ```text
//! Header:      0x0000-0x001F (32 bytes, all zeros)
//! Index Table: 0x0020-0x021F (512 bytes)
//!   Slot 0: 0x0000 → Animation at 0x220
//! Animation Data: 0x0220+
//!   0x0220: 0x0000 0x0001  (Frame 0, duration 1)
//!   0x0224: 0xFFFF 0x0000  (End marker)
//! ```
//!
//! ### AGMAGIC.anm (Complex Example with Jumps)
//!
//! ```text
//! Multiple animation slots with different sequences,
//! including loops (Jump instructions) and shared data regions.
//! Slot 12 and Slot 13 may share overlapping data intentionally.
//! ```
//!
//! # Usage Examples
//!
//! ## Loading an ANM file
//!
//! ```no_run
//! use dvine_types::file::anm::File;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let anm = File::open("AGMAGIC.anm")?;
//!
//! println!("Total animation slots: {}", anm.slot_count());
//!
//! // Get a specific animation sequence
//! if let Some(sequence) = anm.get_sequence(0) {
//!     println!("Animation slot 0 has {} frames", sequence.frames().len());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Iterating over animation sequences
//!
//! ```no_run
//! use dvine_types::file::anm::{File, FrameDescriptor};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let anm = File::open("AGMAGIC.anm")?;
//!
//! for (slot, sequence) in anm.sequences() {
//!     println!("Slot {}: {} frames", slot, sequence.frames().len());
//!     for (i, frame) in sequence.frames().iter().enumerate() {
//!         match frame {
//!             FrameDescriptor::Frame { frame_id, duration } => {
//!                 println!("  Frame {}: ID={}, Duration={}", i, frame_id, duration);
//!             }
//!             FrameDescriptor::End => {
//!                 println!("  Frame {}: END", i);
//!             }
//!             _ => {}
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Creating a new ANM file
//!
//! ```no_run
//! use dvine_types::file::anm::{File, AnimationSequence, FrameDescriptor};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut anm = File::new();
//!
//! // Create a simple animation sequence
//! let mut sequence = AnimationSequence::new();
//! sequence.add_frame(FrameDescriptor::frame(0, 10));
//! sequence.add_frame(FrameDescriptor::frame(1, 10));
//! sequence.add_frame(FrameDescriptor::frame(2, 10));
//! sequence.add_end_marker();
//!
//! // Add to slot 0
//! anm.set_sequence(0, sequence)?;
//!
//! // Save to file
//! anm.save("output.anm")?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Creating a looping animation
//!
//! ```no_run
//! use dvine_types::file::anm::{File, AnimationSequence, FrameDescriptor};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut anm = File::new();
//!
//! // Create a looping animation sequence
//! let mut sequence = AnimationSequence::new();
//! sequence.add_frame(FrameDescriptor::frame(0, 10));
//! sequence.add_frame(FrameDescriptor::frame(1, 10));
//! sequence.add_frame(FrameDescriptor::frame(2, 10));
//! sequence.add_frame(FrameDescriptor::jump(0)); // Jump back to frame 0
//! sequence.add_end_marker();
//!
//! anm.set_sequence(5, sequence)?;
//! anm.save("looping.anm")?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Parsing with custom configuration
//!
//! ```no_run
//! use dvine_types::file::anm::{AnimationSequence, ParseConfig};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let data = std::fs::read("complex.anm")?;
//!
//! // Use lenient parsing for complex animations with many loops
//! let config = ParseConfig::lenient();
//! let (sequence, stats) = AnimationSequence::from_bytes_with_config(&data[0x220..], &config)?;
//!
//! println!(
//!     "Parsed {} frames, visited {} positions",
//!     sequence.len(),
//!     stats.unique_frame_positions
//! );
//! # Ok(())
//! # }
//! ```

// TODO: Potential Enhancements
// 1. **Shared Sequence Optimization**
//    - Detect truly identical sequences and deduplicate in-memory
//    - Maintain original file layout for serialization
// 2. **Better Diagnostics**
//    - Report loop detection details
//    - Suggest fixes for malformed files
// 3. **Validation Mode**
//    - Strict mode that errors on loops instead of stopping
//    - Report all detected issues
// 4. **Performance**
//    - Cache parsed sequences by offset
//    - Lazy parsing for large files

// Module declarations
pub mod constants;
pub mod file;
pub mod frame;
pub mod parse_config;
pub mod sequence;

// Re-exports for convenience
pub use self::file::File;
pub use self::frame::FrameDescriptor;
pub use self::parse_config::ParseConfig;
pub use self::sequence::AnimationSequence;
