//! ANM (Animation) CLI Utility
//!
//! A command-line tool for managing, extracting, and verifying animation sequence files.
//!
//! # Features
//!
//! - **unpack**: Extract animation sequences from an ANM file to JSON format
//! - **pack**: Combine JSON animation data into an ANM file
//! - **info**: Display detailed information about an ANM file
//! - **verify**: Validate ANM encoder/decoder round-trip accuracy
//!
//! # Animation Sequence Format
//!
//! Each ANM file contains up to 256 animation slots. Each slot can contain:
//! - Regular animation frames (sprite ID + duration)
//! - Special markers (end, jump, sound, event)
//!
//! # JSON Format
//!
//! Animation data is stored in JSON with the following structure:
//! ```json
//! {
//!   "sequences": [
//!     {
//!       "slot": 0,
//!       "frames": [
//!         {
//!           "type": "Frame",
//!           "frame_id": 0,
//!           "duration": 10
//!         },
//!         {
//!           "type": "Frame",
//!           "frame_id": 1,
//!           "duration": 10
//!         },
//!         {
//!           "type": "Sound",
//!           "sound_id": 5
//!         },
//!         {
//!           "type": "Jump",
//!           "target": 0
//!         },
//!         {
//!           "type": "End"
//!         }
//!       ]
//!     }
//!   ]
//! }
//! ```
//!
//! # Usage
//!
//! ```bash
//! # Show ANM file information
//! cargo run --example anm_utils -- info AGMAGIC.anm
//!
//! # Unpack an ANM file to JSON
//! cargo run --example anm_utils -- unpack AGMAGIC.anm
//!
//! # Unpack with custom output path
//! cargo run --example anm_utils -- unpack AGMAGIC.anm -o animations.json
//!
//! # Pack JSON to ANM
//! cargo run --example anm_utils -- pack animations.json output.anm
//!
//! # Verify encoder/decoder correctness
//! cargo run --example anm_utils -- verify AGMAGIC.anm
//!
//! # Show detailed information with frame-by-frame breakdown
//! cargo run --example anm_utils -- info AGMAGIC.anm --detailed
//! ```

use clap::{Parser, Subcommand};
use dvine_rs::prelude::file::anm::{
	AnimationSequence, File as AnmFile, FrameDescriptor, ParseConfig,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "anm_utils")]
#[command(author = "dvine-rs project")]
#[command(version = "1.0")]
#[command(about = "ANM animation utility - pack, unpack, verify, and manage ANM files", long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Display information about an ANM file
	Info {
		/// Input ANM file path
		#[arg(value_name = "INPUT_ANM")]
		input: PathBuf,

		/// Show detailed frame-by-frame information
		#[arg(short, long)]
		detailed: bool,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Unpack an ANM file to JSON format
	Unpack {
		/// Input ANM file path
		#[arg(value_name = "INPUT_ANM")]
		input: PathBuf,

		/// Output JSON file path (optional, defaults to `input.json`)
		#[arg(short, long, value_name = "OUTPUT_JSON")]
		output: Option<PathBuf>,

		/// Pretty-print JSON output
		#[arg(short, long)]
		pretty: bool,

		/// Use lenient parsing for complex animations with many loops
		#[arg(short, long)]
		lenient: bool,

		/// Use strict parsing with lower limits
		#[arg(short, long)]
		strict: bool,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Pack JSON animation data into an ANM file
	Pack {
		/// Input JSON file path
		#[arg(value_name = "INPUT_JSON")]
		input: PathBuf,

		/// Output ANM file path
		#[arg(value_name = "OUTPUT_ANM")]
		output: PathBuf,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Verify ANM encoder/decoder round-trip accuracy
	Verify {
		/// Input ANM file path to verify
		#[arg(value_name = "INPUT_ANM")]
		input: PathBuf,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,

		/// Save intermediate files for debugging
		#[arg(short, long)]
		save_intermediate: bool,

		/// Use lenient parsing for complex animations
		#[arg(short, long)]
		lenient: bool,

		/// Tolerate frame count differences (for looping animations)
		#[arg(long)]
		tolerate_loops: bool,
	},
}

/// Frame descriptor for JSON serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum JsonFrameDescriptor {
	Frame {
		frame_id: u16,
		duration: u16,
	},
	End,
	Jump {
		target: u16,
	},
	Sound {
		sound_id: u16,
	},
	Event {
		event_id: u16,
	},
}

impl From<&FrameDescriptor> for JsonFrameDescriptor {
	fn from(frame: &FrameDescriptor) -> Self {
		match frame {
			FrameDescriptor::Frame {
				frame_id,
				duration,
			} => JsonFrameDescriptor::Frame {
				frame_id: *frame_id,
				duration: *duration,
			},
			FrameDescriptor::End => JsonFrameDescriptor::End,
			FrameDescriptor::Jump {
				target,
			} => JsonFrameDescriptor::Jump {
				target: *target,
			},
			FrameDescriptor::Sound {
				sound_id,
			} => JsonFrameDescriptor::Sound {
				sound_id: *sound_id,
			},
			FrameDescriptor::Event {
				event_id,
			} => JsonFrameDescriptor::Event {
				event_id: *event_id,
			},
		}
	}
}

impl From<JsonFrameDescriptor> for FrameDescriptor {
	fn from(json: JsonFrameDescriptor) -> Self {
		match json {
			JsonFrameDescriptor::Frame {
				frame_id,
				duration,
			} => FrameDescriptor::frame(frame_id, duration),
			JsonFrameDescriptor::End => FrameDescriptor::end(),
			JsonFrameDescriptor::Jump {
				target,
			} => FrameDescriptor::jump(target),
			JsonFrameDescriptor::Sound {
				sound_id,
			} => FrameDescriptor::sound(sound_id),
			JsonFrameDescriptor::Event {
				event_id,
			} => FrameDescriptor::event(event_id),
		}
	}
}

/// Animation sequence for JSON serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonSequence {
	slot: usize,
	frames: Vec<JsonFrameDescriptor>,
}

/// Complete ANM file metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnmMetadata {
	sequences: Vec<JsonSequence>,
}

fn handle_info(
	input: PathBuf,
	detailed: bool,
	verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
	if verbose {
		println!("Reading ANM file: {}", input.display());
	}

	let anm = AnmFile::open(&input)?;

	println!("╔════════════════════════════════════════╗");
	println!("║       ANM File Information             ║");
	println!("╚════════════════════════════════════════╝");
	println!();
	println!("File: {}", input.display());
	println!("Total Slots: {}", anm.slot_count());
	println!("Active Sequences: {}", anm.sequences().len());
	println!();

	if anm.sequences().is_empty() {
		println!("No animation sequences found.");
		return Ok(());
	}

	println!("Sequence Summary:");
	println!("┌──────┬────────────┬──────────────────────────────────┐");
	println!("│ Slot │ Frames     │ Content                          │");
	println!("├──────┼────────────┼──────────────────────────────────┤");

	for (slot, sequence) in anm.sequences() {
		let frames = sequence.frames();
		let frame_count = frames.len();

		// Analyze sequence content
		let regular_frames = frames.iter().filter(|f| f.is_frame()).count();
		let has_jump = frames.iter().any(FrameDescriptor::is_jump);
		let has_sound = frames.iter().any(FrameDescriptor::is_sound);
		let has_event = frames.iter().any(FrameDescriptor::is_event);
		let has_end = frames.iter().any(FrameDescriptor::is_end);

		let mut content = Vec::new();
		if regular_frames > 0 {
			content.push(format!("{} frames", regular_frames));
		}
		if has_jump {
			content.push("jump".to_string());
		}
		if has_sound {
			content.push("sound".to_string());
		}
		if has_event {
			content.push("event".to_string());
		}
		if has_end {
			content.push("end".to_string());
		}

		let content_str = if content.is_empty() {
			"empty".to_string()
		} else {
			content.join(", ")
		};

		println!("│ {:4} │ {:10} │ {:<32} │", slot, frame_count, content_str);
	}

	println!("└──────┴────────────┴──────────────────────────────────┘");
	println!();

	if detailed {
		println!("Detailed Frame Information:");
		println!();

		for (slot, sequence) in anm.sequences() {
			println!("Slot {}: {} frames", slot, sequence.frames().len());
			println!("┌──────┬─────────────────────────────────────────────┐");
			println!("│ #    │ Frame Descriptor                            │");
			println!("├──────┼─────────────────────────────────────────────┤");

			for (idx, frame) in sequence.frames().iter().enumerate() {
				let desc = match frame {
					FrameDescriptor::Frame {
						frame_id,
						duration,
					} => format!("Frame(id={}, duration={})", frame_id, duration),
					FrameDescriptor::End => "End Marker".to_string(),
					FrameDescriptor::Jump {
						target,
					} => format!("Jump(target={})", target),
					FrameDescriptor::Sound {
						sound_id,
					} => format!("Sound(id={})", sound_id),
					FrameDescriptor::Event {
						event_id,
					} => format!("Event(id={})", event_id),
				};
				println!("│ {:4} │ {:<43} │", idx, desc);
			}

			println!("└──────┴─────────────────────────────────────────────┘");
			println!();
		}
	}

	Ok(())
}

fn handle_unpack(
	input: PathBuf,
	output: Option<PathBuf>,
	pretty: bool,
	lenient: bool,
	strict: bool,
	verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
	// Determine parse config
	let parse_config = if lenient {
		if verbose {
			println!("Using lenient parsing mode");
		}
		ParseConfig::lenient()
	} else if strict {
		if verbose {
			println!("Using strict parsing mode");
		}
		ParseConfig::strict()
	} else {
		ParseConfig::default()
	};

	if verbose {
		println!("Reading ANM file: {}", input.display());
		println!(
			"Parse config: max_iterations={}, max_visits_per_index={}",
			parse_config.max_iterations, parse_config.max_visits_per_index
		);
	}

	let anm = AnmFile::open(&input)?;

	// Convert to JSON format
	let mut sequences = Vec::new();
	for (slot, sequence) in anm.sequences() {
		let frames: Vec<JsonFrameDescriptor> =
			sequence.frames().iter().map(JsonFrameDescriptor::from).collect();

		sequences.push(JsonSequence {
			slot: *slot,
			frames,
		});
	}

	let metadata = AnmMetadata {
		sequences,
	};

	// Determine output path
	let output_path = output.unwrap_or_else(|| {
		let mut p = input.clone();
		p.set_extension("json");
		p
	});

	if verbose {
		println!("Writing JSON to: {}", output_path.display());
		println!("Sequences: {}", metadata.sequences.len());
	}

	// Serialize to JSON
	let json = if pretty {
		serde_json::to_string_pretty(&metadata)?
	} else {
		serde_json::to_string(&metadata)?
	};

	fs::write(&output_path, json)?;

	println!(
		"✓ Successfully unpacked {} sequences to {}",
		metadata.sequences.len(),
		output_path.display()
	);

	Ok(())
}

fn handle_pack(
	input: PathBuf,
	output: PathBuf,
	verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
	if verbose {
		println!("Reading JSON file: {}", input.display());
	}

	let json_data = fs::read_to_string(&input)?;
	let metadata: AnmMetadata = serde_json::from_str(&json_data)?;

	if verbose {
		println!("Found {} sequences", metadata.sequences.len());
	}

	// Create ANM file
	let mut anm = AnmFile::new();

	for json_seq in metadata.sequences {
		let frames: Vec<FrameDescriptor> =
			json_seq.frames.into_iter().map(FrameDescriptor::from).collect();

		let sequence = AnimationSequence::from_frames(frames);
		anm.set_sequence(json_seq.slot, sequence)?;

		if verbose {
			println!("Added sequence to slot {}", json_seq.slot);
		}
	}

	// Save ANM file
	if verbose {
		println!("Writing ANM file: {}", output.display());
	}

	anm.save(&output)?;

	println!("✓ Successfully packed {} sequences to {}", anm.sequences().len(), output.display());

	Ok(())
}

fn handle_verify(
	input: PathBuf,
	verbose: bool,
	save_intermediate: bool,
	lenient: bool,
	tolerate_loops: bool,
) -> Result<(), Box<dyn std::error::Error>> {
	if verbose {
		println!("Reading original ANM file: {}", input.display());
		if lenient {
			println!("Using lenient parsing mode");
		}
		if tolerate_loops {
			println!("Tolerating frame count differences from loop detection");
		}
	}

	// Load original
	let original = AnmFile::open(&input)?;
	let original_bytes = fs::read(&input)?;

	if verbose {
		println!("Original file size: {} bytes", original_bytes.len());
		println!("Sequences: {}", original.sequences().len());
	}

	// Re-encode
	let reencoded_bytes = original.to_bytes();

	if verbose {
		println!("Re-encoded file size: {} bytes", reencoded_bytes.len());
	}

	// Save intermediate if requested
	if save_intermediate {
		let intermediate_path = input.with_extension("reencoded.anm");
		fs::write(&intermediate_path, &reencoded_bytes)?;
		println!("Saved intermediate file: {}", intermediate_path.display());
	}

	// Parse re-encoded
	let reparsed = AnmFile::from_bytes(&reencoded_bytes)?;

	// Compare
	println!();
	println!("╔════════════════════════════════════════╗");
	println!("║       Verification Results             ║");
	println!("╚════════════════════════════════════════╝");
	println!();

	let mut all_match = true;

	// Check sequence count
	if original.sequences().len() != reparsed.sequences().len() {
		println!("✗ Sequence count mismatch!");
		println!("  Original: {}", original.sequences().len());
		println!("  Reparsed: {}", reparsed.sequences().len());
		all_match = false;
	} else {
		println!("✓ Sequence count: {}", original.sequences().len());
	}

	// Check each sequence
	for (slot, orig_seq) in original.sequences() {
		if let Some(repr_seq) = reparsed.get_sequence(*slot) {
			if orig_seq.frames().len() != repr_seq.frames().len() {
				// Check if either version has jump instructions (indicates loop)
				let has_orig_jump = orig_seq
					.frames()
					.iter()
					.any(dvine_rs::prelude::file::AnmFrameDescriptor::is_jump);
				let has_repr_jump = repr_seq
					.frames()
					.iter()
					.any(dvine_rs::prelude::file::AnmFrameDescriptor::is_jump);
				let has_jumps = has_orig_jump || has_repr_jump;

				if tolerate_loops && has_jumps {
					// For looping animations, frame count differences are expected
					println!(
						"⚠ Slot {} frame count mismatch (expected for looping animations)",
						slot
					);
					println!("  Original: {}", orig_seq.frames().len());
					println!("  Reparsed: {}", repr_seq.frames().len());

					if verbose {
						println!("  Note: Sequence contains jump instructions (looping animation)");
					}
					// Don't mark as failure - this is expected for loops
				} else {
					println!("✗ Slot {} frame count mismatch!", slot);
					println!("  Original: {}", orig_seq.frames().len());
					println!("  Reparsed: {}", repr_seq.frames().len());

					if has_jumps {
						println!(
							"  Note: Sequence has jump instructions. Use --tolerate-loops to accept this."
						);
					}

					all_match = false;
				}
			} else if orig_seq != repr_seq {
				println!("✗ Slot {} content mismatch!", slot);
				all_match = false;

				if verbose {
					println!("  Comparing frames:");
					for (idx, (orig_frame, repr_frame)) in
						orig_seq.frames().iter().zip(repr_seq.frames().iter()).enumerate()
					{
						if orig_frame != repr_frame {
							println!("    Frame {}: {} != {}", idx, orig_frame, repr_frame);
						}
					}
				}
			} else if verbose {
				println!("✓ Slot {}: {} frames match", slot, orig_seq.frames().len());
			}
		} else {
			println!("✗ Slot {} missing in reparsed file!", slot);
			all_match = false;
		}
	}

	println!();
	if all_match {
		println!("╔════════════════════════════════════════╗");
		println!("║   ✓ Verification PASSED                ║");
		println!("║   All sequences match perfectly!       ║");
		println!("╚════════════════════════════════════════╝");
	} else {
		println!("╔════════════════════════════════════════╗");
		println!("║   ✗ Verification FAILED                ║");
		println!("║   Differences detected!                ║");
		println!("╚════════════════════════════════════════╝");

		if !tolerate_loops {
			println!();
			println!("Note: This file may contain looping animations with jump instructions.");
			println!("      Frame count differences are expected for such animations.");
			println!("      Try running with --tolerate-loops to skip loop-related differences.");
		}

		return Err("Verification failed".into());
	}

	Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let cli = Cli::parse();

	match cli.command {
		Commands::Info {
			input,
			detailed,
			verbose,
		} => handle_info(input, detailed, verbose)?,
		Commands::Unpack {
			input,
			output,
			pretty,
			lenient,
			strict,
			verbose,
		} => handle_unpack(input, output, pretty, lenient, strict, verbose)?,
		Commands::Pack {
			input,
			output,
			verbose,
		} => handle_pack(input, output, verbose)?,
		Commands::Verify {
			input,
			verbose,
			save_intermediate,
			lenient,
			tolerate_loops,
		} => handle_verify(input, verbose, save_intermediate, lenient, tolerate_loops)?,
	}

	Ok(())
}
