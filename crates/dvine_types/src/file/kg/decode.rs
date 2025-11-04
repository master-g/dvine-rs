use super::{Compression, Header, KgError, opcodes};

/// State structure for the decompressor
#[derive(Debug)]
struct DecompressorState {
	output_buffer: Vec<u8>,
	compressed_data: Vec<u8>,
	read_offset: usize,
	write_position: usize,
	total_size: usize,
	line_bytes: usize,
	bytes_per_pixel: usize,
	bit_buffer: u8,
	bits_remaining: u32,
	lru_cache: [[u8; 8]; 256],
	palette: [[u8; 4]; 256],
}

impl DecompressorState {
	fn new(width: usize, height: usize, bytes_per_pixel: usize, compressed_data: Vec<u8>) -> Self {
		let total_size = width * height * bytes_per_pixel;
		let line_bytes = width * bytes_per_pixel;

		Self {
			output_buffer: vec![0; total_size],
			compressed_data,
			read_offset: 0,
			write_position: 0,
			total_size,
			line_bytes,
			bytes_per_pixel,
			bit_buffer: 0,
			bits_remaining: 0,
			lru_cache: [[0; 8]; 256],
			palette: [[0; 4]; 256],
		}
	}

	fn read_bits(&mut self, num_bits: u32) -> u32 {
		let mut edx = u32::from(self.bit_buffer);
		let mut ebx = self.bits_remaining;
		let mut eax = num_bits;

		if ebx < num_bits {
			loop {
				let shift_bits = ebx;
				edx <<= shift_bits;
				eax -= ebx;

				let new_byte = self.compressed_data[self.read_offset];
				self.read_offset += 1;
				edx = (edx & 0xFFFF_FF00) | u32::from(new_byte);
				ebx = 8;

				let ecx_loop = eax;
				if ebx >= ecx_loop {
					edx <<= ecx_loop;
					break;
				}
			}
			ebx -= eax;
		} else {
			edx <<= num_bits;
			ebx -= num_bits;
		}

		self.bit_buffer = edx as u8;
		edx >>= 8;
		self.bits_remaining = ebx;

		edx
	}

	fn read_variable_length(&mut self) -> u32 {
		let value = self.read_bits(2);
		if value != 0 {
			return value;
		}

		let value = self.read_bits(4);
		if value != 0 {
			return value + 3;
		}

		let value = self.read_bits(8);
		if value != 0 {
			return value;
		}

		let value = self.read_bits(16);
		if value != 0 {
			return value;
		}

		let high = self.read_bits(16);
		let low = self.read_bits(16);
		(high << 16) | low
	}

	fn read_opcode(&mut self) -> u8 {
		const OPCODE_TABLE: [u8; 4] = [12, 13, 14, 15];

		let bit1 = self.read_bits(1);
		if bit1 == 0 {
			return 0;
		}

		let bit2 = self.read_bits(1);
		if bit2 == 0 {
			return 2;
		}

		let index = self.read_bits(2) as usize;
		OPCODE_TABLE[index]
	}

	fn update_lru_cache(&mut self, reference_color: u8, new_color: u8) {
		let cache_entry = &mut self.lru_cache[reference_color as usize];

		let mut position = 8;
		for (i, &color) in cache_entry.iter().enumerate() {
			if color == new_color {
				position = i;
				break;
			}
		}

		if position == 0 {
			return;
		}
		if position == 8 {
			position = 7;
		}

		for i in (1..=position).rev() {
			cache_entry[i] = cache_entry[i - 1];
		}
		cache_entry[0] = new_color;
	}

	fn read_color_index(&mut self) -> u8 {
		let flag = self.read_bits(1);

		if flag == 1 {
			self.read_bits(8) as u8
		} else {
			let ref_pos = self.write_position.saturating_sub(self.bytes_per_pixel);
			let reference_color = self.output_buffer[ref_pos];
			let cache_index = self.read_bits(3) as usize;
			self.lru_cache[reference_color as usize][cache_index]
		}
	}

	fn write_pixel(&mut self, value: u8) {
		self.output_buffer[self.write_position] = value;
	}

	fn copy_data(&mut self, dst_offset: usize, src_offset: usize, length: usize) {
		if self.bytes_per_pixel == 1 {
			for i in 0..length {
				self.output_buffer[dst_offset + i] = self.output_buffer[src_offset + i];
			}
			self.write_position += length;
		} else {
			let total_bytes = self.bytes_per_pixel * length;
			self.write_position += total_bytes;

			let mut ecx = dst_offset;
			let mut esi = src_offset;
			let mut edi = length;

			while edi > 0 {
				let dl = self.output_buffer[esi];
				self.output_buffer[ecx] = dl;
				ecx += self.bytes_per_pixel;
				esi += self.bytes_per_pixel;
				edi -= 1;
			}
		}
	}

	fn opcode_0_dictionary_lookup(&mut self) {
		let color_index = self.read_color_index();
		self.write_pixel(color_index);

		let ref_pos = self.write_position.saturating_sub(self.bytes_per_pixel);
		let reference_color = self.output_buffer[ref_pos];

		self.update_lru_cache(reference_color, color_index);
		self.write_position += self.bytes_per_pixel;
	}

	fn opcode_2_copy_previous_pixel(&mut self) {
		let length = self.read_variable_length() as usize;
		let src = self.write_position.saturating_sub(self.bytes_per_pixel);
		self.copy_data(self.write_position, src, length);
	}

	fn opcode_12_copy_up_1_line(&mut self) {
		let length = self.read_variable_length() as usize;
		let src = self.write_position.saturating_sub(self.line_bytes);
		self.copy_data(self.write_position, src, length);
	}

	fn opcode_13_copy_left_up(&mut self) {
		let length = self.read_variable_length() as usize;
		let src = if self.write_position > self.line_bytes {
			self.write_position - self.line_bytes + self.bytes_per_pixel
		} else {
			self.bytes_per_pixel
		};
		self.copy_data(self.write_position, src, length);
	}

	fn opcode_14_copy_up_2_lines(&mut self) {
		let length = self.read_variable_length() as usize;
		let src = if self.write_position > (self.line_bytes + self.bytes_per_pixel) {
			self.write_position - self.line_bytes - self.bytes_per_pixel
		} else {
			0
		};
		self.copy_data(self.write_position, src, length);
	}

	fn opcode_15_copy_up_double(&mut self) {
		let length = self.read_variable_length() as usize;
		let src = self.write_position.saturating_sub(self.bytes_per_pixel * 2);
		self.copy_data(self.write_position, src, length);
	}

	fn decompress_type1(&mut self) -> bool {
		// Read first 2 bytes
		for _ in 0..2 {
			let byte_val = self.read_bits(8) as u8;
			self.write_pixel(byte_val);
			self.write_position += self.bytes_per_pixel;
		}

		// Main loop
		while self.write_position < self.total_size {
			let opcode = self.read_opcode();

			match opcode {
				opcodes::OP_DICT_LOOKUP => self.opcode_0_dictionary_lookup(),
				opcodes::OP_COPY_PREV_PIXEL => self.opcode_2_copy_previous_pixel(),
				opcodes::OP_COPY_PREV_LINE => self.opcode_12_copy_up_1_line(),
				opcodes::OP_COPY_DIAGONAL_1 => self.opcode_13_copy_left_up(),
				opcodes::OP_COPY_DIAGONAL_2 => self.opcode_14_copy_up_2_lines(),
				opcodes::OP_COPY_DOUBLE_BPP => self.opcode_15_copy_up_double(),
				_ => {}
			}
		}

		true
	}
}

fn load_palette(data: &[u8], header: &Header) -> Option<[[u8; 4]; 256]> {
	if header.compression_type() != Compression::BPP3 {
		return None;
	}
	if header.palette_offset == 0 {
		return None;
	}

	let mut palette = [[0u8; 4]; 256];
	let palette_offset = header.palette_offset as usize;

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

	Some(palette)
}

fn apply_palette(indexed_data: &[u8], palette: Option<&[[u8; 4]; 256]>) -> Vec<u8> {
	let Some(pal) = palette else {
		return indexed_data.to_vec();
	};

	let mut rgb_data = Vec::with_capacity(indexed_data.len() * 3);
	for &index in indexed_data {
		let color = &pal[index as usize];
		rgb_data.push(color[0]);
		rgb_data.push(color[1]);
		rgb_data.push(color[2]);
	}

	rgb_data
}

/// Decompress KG format data from a byte slice
/// Returns (Header, RGB data)
pub fn decompress(data: &[u8]) -> Result<super::File, KgError> {
	let header = Header::from_bytes(data)?;

	let compression_type = header.compression_type();

	let padding =
		header.padding_size().map(|size| data[Header::SIZE..Header::SIZE + size].to_vec());

	let palette = load_palette(data, &header);
	let has_palette = palette.is_some();

	let output_bpp = if compression_type == Compression::BPP3 && has_palette {
		1
	} else if compression_type == Compression::BPP3 {
		3
	} else {
		1
	};

	let width = header.width as usize;
	let height = header.height as usize;
	let data_offset = header.data_offset as usize;

	let compressed_data = data[data_offset..].to_vec();

	let mut state = DecompressorState::new(width, height, output_bpp, compressed_data);
	if let Some(pal) = palette {
		state.palette = pal;
	}

	let success = if compression_type == Compression::BPP3 {
		state.decompress_type1()
	} else {
		false
	};

	if !success {
		return Err(KgError::DecompressionError);
	}

	let final_data = if has_palette {
		apply_palette(&state.output_buffer, Some(&state.palette))
	} else {
		state.output_buffer
	};

	Ok(super::File {
		header,
		padding,
		palette,
		pixels: final_data,
	})
}
