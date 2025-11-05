//! Constants used in `.EFC` files

/// Maximum number of effects supported in `.EFC` files
pub const MAX_EFFECTS: usize = 256;

/// Size of the index table in bytes (256 entries × 4 bytes each)
pub const INDEX_TABLE_SIZE: usize = MAX_EFFECTS * 4;

/// Size of the sound data header in bytes
pub const SOUND_HEADER_SIZE: usize = 4;

/// Size of the ADPCM data header in bytes
pub const ADPCM_HEADER_SIZE: usize = 0xC0;

/// Offset of `sample_count` field within ADPCM header
pub const SAMPLE_COUNT_OFFSET: usize = 0xBC;

/// Number of entries in the IMA ADPCM step table
pub const STEP_TABLE_ENTRIES: usize = 89;
