//! Benchmark suite for KG file decoding
//!
//! This benchmark measures the performance of KG file decompression
//! and helps identify hot paths in the decoder.
//!
//! Run with: cargo bench --manifest-path benches/Cargo.toml
//!
//! For flamegraph profiling:
//! cargo bench --manifest-path benches/Cargo.toml -- --profile-time=5

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use dvine_types::file::kg::File;
use std::{fs, hint::black_box};

/// Benchmark KG decompression with real game files
fn bench_decompress_real_files(c: &mut Criterion) {
	let mut group = c.benchmark_group("kg_decompress_real");

	// Real game files - paths are relative to workspace root
	let test_files = vec![
		("BLACK", concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/BLACK")),
		("VYADOY01", concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/VYADOY01")),
	];

	for (name, path) in test_files {
		// Try to load the file, skip if not found
		let data = match fs::read(path) {
			Ok(d) => d,
			Err(_) => {
				eprintln!("Warning: Could not find test file: {}", path);
				continue;
			}
		};

		// Parse header to get actual dimensions
		let header = match dvine_types::file::kg::Header::from_bytes(&data) {
			Ok(h) => h,
			Err(_) => {
				eprintln!("Warning: Could not parse header for: {}", name);
				continue;
			}
		};

		let pixels = (header.width() as u64) * (header.height() as u64);
		group.throughput(Throughput::Elements(pixels));
		group.bench_with_input(BenchmarkId::new("decompress", name), &data, |b, data| {
			b.iter(|| {
				let result = File::from_reader(&mut black_box(data).as_slice());
				black_box(result)
			});
		});
	}

	group.finish();
}

/// Benchmark header parsing separately
fn bench_header_parsing(c: &mut Criterion) {
	use dvine_types::file::kg::Header;

	let mut group = c.benchmark_group("kg_header");

	// Use real test file
	let data = match fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/BLACK")) {
		Ok(d) => d,
		Err(_) => {
			eprintln!("Warning: Could not find test file for header benchmark");
			return;
		}
	};

	group.bench_function("parse_header", |b| {
		b.iter(|| {
			let result = Header::from_bytes(black_box(&data));
			black_box(result)
		});
	});

	group.finish();
}

/// Benchmark palette loading
fn bench_palette_loading(c: &mut Criterion) {
	let mut group = c.benchmark_group("kg_palette");

	// Use real test file
	let data = match fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/BLACK")) {
		Ok(d) => d,
		Err(_) => {
			eprintln!("Warning: Could not find test file for palette benchmark");
			return;
		}
	};

	group.bench_function("load_palette", |b| {
		b.iter(|| {
			// This benchmarks the palette extraction logic
			let header = dvine_types::file::kg::Header::from_bytes(&data).unwrap();
			let palette_offset = header.palette_offset() as usize;

			let mut palette = [[0u8; 4]; 256];
			for (i, color) in palette.iter_mut().enumerate() {
				let offset = palette_offset + i * 4;
				if offset + 3 < data.len() {
					let b = data[offset];
					let g = data[offset + 1];
					let r = data[offset + 2];
					color[0] = r;
					color[1] = g;
					color[2] = b;
					color[3] = 0;
				}
			}
			black_box(palette)
		});
	});

	group.finish();
}

/// Benchmark palette application (indexed to RGB conversion)
fn bench_palette_application(c: &mut Criterion) {
	let mut group = c.benchmark_group("kg_palette_apply");

	// Create test indexed data and palette
	let indexed_data: Vec<u8> = (0..65536).map(|i| (i % 256) as u8).collect();
	let palette: [[u8; 4]; 256] = core::array::from_fn(|i| {
		let i = i as u8;
		[i, i.wrapping_mul(2), i.wrapping_mul(3), 0]
	});

	group.throughput(Throughput::Elements(indexed_data.len() as u64));
	group.bench_function("indexed_to_rgb", |b| {
		b.iter(|| {
			let mut rgb_data = Vec::with_capacity(indexed_data.len() * 3);
			for &index in &indexed_data {
				let color = &palette[index as usize];
				rgb_data.push(color[0]);
				rgb_data.push(color[1]);
				rgb_data.push(color[2]);
			}
			black_box(rgb_data)
		});
	});

	group.finish();
}

/// Benchmark bit reading operations
fn bench_bit_operations(c: &mut Criterion) {
	let mut group = c.benchmark_group("kg_bit_ops");

	// Simulate bit buffer operations
	let compressed_data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();

	group.bench_function("read_bits_simulation", |b| {
		b.iter(|| {
			let mut bit_buffer: u8 = 0;
			let mut bits_remaining: u32 = 0;
			let mut read_offset = 0;
			let mut total_read = 0u32;

			// Simulate reading bits like in the decompressor
			for _ in 0..1000 {
				let num_bits = 3u32; // Typical read size
				let mut edx = u32::from(bit_buffer);
				let mut ebx = bits_remaining;

				if ebx < num_bits {
					edx <<= ebx;
					if read_offset < compressed_data.len() {
						let new_byte = compressed_data[read_offset];
						read_offset += 1;
						edx = (edx & 0xFFFF_FF00) | u32::from(new_byte);
						ebx = 8;
						edx <<= num_bits;
					}
					ebx = ebx.saturating_sub(num_bits);
				} else {
					edx <<= num_bits;
					ebx -= num_bits;
				}

				bit_buffer = edx as u8;
				edx >>= 8;
				bits_remaining = ebx;
				total_read = total_read.wrapping_add(edx);
			}

			black_box(total_read)
		});
	});

	group.finish();
}

/// Benchmark LRU cache update (our optimized version)
fn bench_lru_cache_update(c: &mut Criterion) {
	let mut group = c.benchmark_group("kg_lru_cache");

	group.bench_function("update_cache_optimized", |b| {
		b.iter(|| {
			let mut lru_cache = [[0u8; 8]; 256];
			// Simulate typical cache update patterns
			for i in 0..1000 {
				let reference_color = (i % 256) as u8;
				let new_color = ((i + 1) % 256) as u8;

				let cache_entry = &mut lru_cache[reference_color as usize];
				let position = cache_entry.iter().position(|&color| color == new_color);

				match position {
					Some(0) => {}
					Some(pos) => {
						cache_entry.copy_within(0..pos, 1);
						cache_entry[0] = new_color;
					}
					None => {
						cache_entry.copy_within(0..7, 1);
						cache_entry[0] = new_color;
					}
				}
			}

			black_box(lru_cache[0][0])
		});
	});

	// Compare with old manual implementation
	group.bench_function("update_cache_manual", |b| {
		b.iter(|| {
			let mut lru_cache = [[0u8; 8]; 256];
			for i in 0..1000 {
				let reference_color = (i % 256) as u8;
				let new_color = ((i + 1) % 256) as u8;

				let cache_entry = &mut lru_cache[reference_color as usize];

				let mut position = 8;
				for (idx, &color) in cache_entry.iter().enumerate() {
					if color == new_color {
						position = idx;
						break;
					}
				}

				if position == 0 {
					continue;
				}
				if position == 8 {
					position = 7;
				}

				for idx in (1..=position).rev() {
					cache_entry[idx] = cache_entry[idx - 1];
				}
				cache_entry[0] = new_color;
			}

			black_box(lru_cache[0][0])
		});
	});

	group.finish();
}

/// Benchmark memory copying patterns used in decompression
fn bench_copy_operations(c: &mut Criterion) {
	let mut group = c.benchmark_group("kg_copy_ops");

	group.bench_function("copy_within_small", |b| {
		let mut buffer = vec![0u8; 1024 * 1024];
		b.iter(|| {
			// Simulate small copies (typical in KG decompression)
			for i in 0..1000 {
				let dst = i * 8 + 8;
				let src = i * 8;
				if dst + 8 <= buffer.len() {
					buffer.copy_within(src..src + 8, dst);
				}
			}
			black_box(buffer.len())
		});
	});

	group.bench_function("copy_within_medium", |b| {
		let mut buffer = vec![0u8; 1024 * 1024];
		b.iter(|| {
			// Simulate medium copies (line copies)
			for i in 0..100 {
				let dst = i * 256 + 256;
				let src = i * 256;
				if dst + 256 <= buffer.len() {
					buffer.copy_within(src..src + 256, dst);
				}
			}
			black_box(buffer.len())
		});
	});

	group.finish();
}

/// Full end-to-end benchmark with realistic data
fn bench_realistic_workload(c: &mut Criterion) {
	let mut group = c.benchmark_group("kg_realistic");

	// Use real game file
	let data = match fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/VYADOY01")) {
		Ok(d) => d,
		Err(_) => {
			eprintln!("Warning: Could not find VYADOY01 for realistic workload benchmark");
			return;
		}
	};

	let header = match dvine_types::file::kg::Header::from_bytes(&data) {
		Ok(h) => h,
		Err(_) => {
			eprintln!("Warning: Could not parse header for realistic workload");
			return;
		}
	};

	let pixels = (header.width() as u64) * (header.height() as u64);

	group.throughput(Throughput::Bytes(data.len() as u64));
	group.sample_size(50); // Fewer samples for larger workload

	group.bench_function("full_decode_pipeline", |b| {
		b.iter(|| {
			let result = File::from_reader(&mut black_box(&data).as_slice());
			black_box(result)
		});
	});

	group.finish();

	// Print summary statistics
	println!("\n=== Benchmark Summary ===");
	println!("Image size: {}x{} ({} pixels)", header.width(), header.height(), pixels);
	println!("Compressed size: {} bytes", data.len());
	println!("Expected output: {} bytes (RGB)", pixels * 3);
}

criterion_group!(
	benches,
	bench_decompress_real_files,
	bench_header_parsing,
	bench_palette_loading,
	bench_palette_application,
	bench_bit_operations,
	bench_lru_cache_update,
	bench_copy_operations,
	bench_realistic_workload,
);

criterion_main!(benches);
