use std::f32;
use std::io::IoResult;
use std::cmp;
use std::slice;
use std::iter::range_step;
use std::default::Default;

use collections::smallintmap::SmallIntMap;
use colortype;
//Markers
//Baseline DCT
static SOF0: u8 = 0xC0;
//Progressive DCT
static SOF2: u8 = 0xC2;
//Huffman Tables
static DHT: u8 = 0xC4;
//Restart Interval start and End (standalone)
static RST0: u8 = 0xD0;
static RST7: u8 = 0xD7;
//Start of Image (standalone)
static SOI: u8 = 0xD8;
//End of image (standalone)
static EOI: u8 = 0xD9;
//Start of Scan
static SOS: u8 = 0xDA;
//Quantization Tables
static DQT: u8 = 0xDB;
//Number of lines
static DNL: u8 = 0xDC;
//Restart Interval
static DRI: u8 = 0xDD;
//Application segments start and end
static APP0: u8 = 0xE0;
static APPF: u8 = 0xEF;
//Comment
static COM: u8 = 0xFE;
//Reserved
static TEM: u8 = 0x01;

static UNZIGZAG: &'static [u8] = &[
	 0,  1,  8, 16,  9,  2,  3, 10,
	17, 24, 32, 25, 18, 11,  4,  5,
	12, 19, 26, 33, 40, 48, 41, 34,
	27, 20, 13,  6,  7, 14, 21, 28,
	35, 42, 49, 56, 57, 50, 43, 36,
	29, 22, 15, 23, 30, 37, 44, 51,
	58, 59, 52, 45, 38, 31, 39, 46,
	53, 60, 61, 54, 47, 55, 62, 63,
];

#[deriving(Default, Clone)]
struct HuffTable {
	lut: ~[(u8, u8)],
	huffval: ~[u8],
	maxcode: ~[int],
	mincode: ~[int],
	valptr: ~[int]
}

#[deriving(Clone)]
struct Component {
	id: u8,
    h: u8,
    v: u8,
    tq: u8,
    dc_table: u8,
    ac_table: u8,
    dc_pred: i32
}

#[deriving(Eq)]
enum JPEGState {
	Start,
	HaveSOI,
	HaveFirstFrame,
	HaveFirstScan,
	End
}

pub struct JPEGDecoder<R> {
	r: R,
	
	qtables: [u8, ..64 * 4],
	dctables: [HuffTable, ..2],
	actables: [HuffTable, ..2],

	h: HuffDecoder,

	height: u16,
	width: u16,
	
	num_components: u8,
	scan_components: ~[u8],
	components: SmallIntMap<Component>,
	
	mcu_row: ~[u8],
	mcu: ~[i32],
	mcu_h: u8,
	mcu_v: u8,

	interval: u16,
	mcucount: u16,
	expected_rst: u8,	
	
	row_count: u8,
	state: JPEGState,
}

impl<R: Reader>JPEGDecoder<R> {
	pub fn new(r: R) -> JPEGDecoder<R> {
		let h: HuffTable  = Default::default();
		
		JPEGDecoder {
			r: r,
	
			qtables: [0u8, ..64 * 4],
			dctables: [h.clone(), h.clone()],
			actables: [h.clone(), h.clone()],

			h: HuffDecoder::new(),

			height: 0,
			width: 0,
			
			num_components: 0,
			scan_components: ~[],
			components: SmallIntMap::new(),
			
			mcu_row: ~[],
			mcu: ~[],
			mcu_h: 0,
			mcu_v: 0,

			interval: 0,
			mcucount: 0,
			expected_rst: RST0,	
			
			row_count: 0,
			state: Start,
		}
	}

	pub fn dimensions(&self) -> (u32, u32) {
		(self.width as u32, self.height as u32)
	}

	pub fn color_type(&self) -> colortype::ColorType {
		if self.num_components == 1 {colortype::Grey(8)} 
		else {colortype::RGB(8)}
	}

	pub fn rowlen(&self) -> uint {
		self.width as uint * self.num_components as uint
	}
	
	pub fn read_scanline(&mut self, buf: &mut [u8]) -> IoResult<uint> {
		if self.state == Start {
			let _ = try!(self.read_metadata());
		}

		if self.row_count == 0 {
			let _ = try!(self.decode_mcu_row());
		}
		
		let w = 8 * ((self.width as uint + 7) / 8);
		let len = w * self.num_components as uint;
		
		let slice = self.mcu_row.slice(self.row_count as uint * len,
								       self.row_count as uint * len + buf.len());
		
		slice::bytes::copy_memory(buf, slice);
		self.row_count = (self.row_count + 1) % (self.mcu_v * 8);
				
		Ok(buf.len())
	}

	pub fn decode_image(&mut self) -> IoResult<~[u8]> {
		if self.state == Start {
			let _ = try!(self.read_metadata());
		}

		let row = self.rowlen();
		let mut buf = slice::from_elem(row * self.height as uint, 0u8);
		
		for chunk in buf.mut_chunks(row) {
			let _len = try!(self.read_scanline(chunk));
		}

		Ok(buf)
	}

	fn decode_mcu_row(&mut self) -> IoResult<()> {
		let w 	  = 8 * ((self.width as uint + 7) / 8);
		let bpp   = self.num_components as uint; 

		for x0 in range_step(0, w * bpp, bpp * 8 * self.mcu_h as uint) {
			let _ = try!(self.decode_mcu());
			upsample_mcu(self.mcu_row, x0, w, bpp, self.mcu, self.mcu_h, self.mcu_v)
		}

		Ok(())
	}

	fn decode_mcu(&mut self) -> IoResult<()> {
		let mut i = 0;

		for k in self.mcu.mut_iter() {
			*k = 0;
		}

		let tmp = self.scan_components.clone();
		for id in tmp.iter() {
			let mut c = self.components.find(&(*id as uint)).unwrap().clone();

			for _ in range(0, c.h * c.v) {
				let pred  = try!(self.decode_block(i, c.dc_table, c.dc_pred, c.ac_table, c.tq));
				c.dc_pred = pred;
				
				i += 1;
			}

			self.components.insert(*id as uint, c); 
		}
		
		self.mcucount += 1;
		self.read_restart()
	}

	fn decode_block(&mut self, i: uint, dc: u8, pred: i32, ac: u8, q: u8) -> IoResult<i32> {
		let zz   = self.mcu.mut_slice(i * 64, i * 64 + 64);
		
		let dctable = &self.dctables[dc];
		let actable = &self.actables[ac];
		let qtable  = self.qtables.slice(64 * q as uint, 
									 	 64 * q as uint + 64);
		
		let t     = try!(self.h.decode_symbol(&mut self.r, dctable));
		let diff  = if t > 0 {try!(self.h.receive(&mut self.r, t))}
					else {0};
		
		//Section F.2.1.3.1
		let diff = extend(diff, t);
		let dc_coeff = diff + pred;

		zz[0] = dc_coeff * qtable[0] as i32;

		let mut k = 0;

		while k < 63 {
			let rs = try!(self.h.decode_symbol(&mut self.r, actable));
			
			let ssss = rs & 0x0F;
			let rrrr = rs >> 4;
			
			if ssss == 0 {
				if rrrr != 15 {
					break
				}
				k += 16;
			}
			else {
				k += rrrr;
				
				//Figure F.14
				let t = try!(self.h.receive(&mut self.r, ssss));
				zz[UNZIGZAG[k + 1]] = extend(t, ssss) * qtable[k + 1] as i32;
				k += 1;
			}
		}

		let a = slow_idct(zz);
		for (i, v) in a.move_iter().enumerate() {
			zz[i] = v;
		}
		level_shift(zz);
		
		Ok(dc_coeff)
	}
	
	fn read_metadata(&mut self) -> IoResult<()> {
		while self.state != HaveFirstScan {
			let byte = try!(self.r.read_u8());

			if byte != 0xFF {
				continue;
			}

			let marker = try!(self.r.read_u8());

			match marker {
				SOI => self.state = HaveSOI,
				DHT => try!(self.read_huffman_tables()),
				DQT => try!(self.read_quantization_tables()),
				SOF0 => {
					let _ = try!(self.read_frame_header());
					self.state = HaveFirstFrame;
				}
				SOS => {
					let _ = try!(self.read_scan_header());
					self.state = HaveFirstScan;
				}				
				DRI => try!(self.read_restart_interval()),
				APP0 .. APPF | COM => {
					let length = try!(self.r.read_be_u16());
					let _ = try!(self.r.read_exact((length -2) as uint));
				}
				TEM  => continue,
				SOF2 => fail!("Progressive DCT unimplemented"),		
				DNL  => fail!("DNL not supported"),
				a    => fail!(format!("unexpected marker {:X}\n", a))
			}
		}
		
		Ok(())
	}

	fn read_frame_header(&mut self) -> IoResult<()> {
		let _frame_length = try!(self.r.read_be_u16());
		
		let sample_precision = try!(self.r.read_u8());
		assert!(sample_precision == 8);
		
		self.height 		  = try!(self.r.read_be_u16());
		self.width  		  = try!(self.r.read_be_u16());
		self.num_components   = try!(self.r.read_u8());

		if self.height == 0 {
			fail!("DNL not supported")
		}

		if self.num_components != 1 && self.num_components != 3 {
			fail!(format!("unsupported number of components: {}", self.num_components))
		}

		self.read_frame_components(self.num_components)
	}

	fn read_frame_components(&mut self, n: u8) -> IoResult<()> {
		let mut blocks_per_mcu = 0;
		for _ in range(0, n) {
			let id = try!(self.r.read_u8());
			let hv = try!(self.r.read_u8());
			let tq = try!(self.r.read_u8());
		
			let c = Component {
				id: id,
				h:  hv >> 4,
				v:  hv & 0x0F,
				tq: tq,
				dc_table: 0,
				ac_table: 0,
				dc_pred: 0
			};

			blocks_per_mcu += (hv >> 4) * (hv & 0x0F);
			self.components.insert(id as uint, c);
		}
		
		let (hmax, vmax) = self.components.iter().fold((0, 0), |(h, v), (_, c)| {
			(cmp::max(h, c.h), cmp::max(v, c.v))
		});

		self.mcu_h = hmax;
		self.mcu_v = vmax;

		//only 1 component no interleaving
		if n == 1 {
			for (_, c) in self.components.mut_iter() {
				c.h = 1;
				c.v = 1;
			}

			blocks_per_mcu = 1;
			self.mcu_h = 1;
			self.mcu_v = 1;
		}

		self.mcu =  slice::from_elem(blocks_per_mcu as uint * 64, 0i32);
		let mcu_row_len  = self.mcu_v as uint * 8 * 8 * ((self.width as uint + 7) / 8) * n as uint;
		self.mcu_row = slice::from_elem(mcu_row_len, 0u8);
		
		Ok(())
	}

	fn read_scan_header(&mut self) -> IoResult<()> {
		let _scan_length = try!(self.r.read_be_u16());

		let num_scan_components = try!(self.r.read_u8());

		self.scan_components = ~[];
		for _ in range(0, num_scan_components as uint) {
			let id = try!(self.r.read_u8());
			let tables = try!(self.r.read_u8());

			let c = self.components.find_mut(&(id as uint)).unwrap();

			c.dc_table = tables >> 4;
			c.ac_table = tables & 0x0F;
		
			self.scan_components.push(id);
		}

		let _spectral_end   = try!(self.r.read_u8());
		let _spectral_start = try!(self.r.read_u8());
		
		let approx = try!(self.r.read_u8());
		
		let _approx_high = approx >> 4;
		let _approx_low  = approx & 0x0F;  
		
		Ok(())
	}

	fn read_quantization_tables(&mut self) -> IoResult<()> {
		let mut table_length = try!(self.r.read_be_u16()) as i32;
		table_length -= 2;

		while table_length > 0 {
			let pqtq = try!(self.r.read_u8());
			let pq = pqtq >> 4;
			let tq = pqtq & 0x0F;

			assert!(pq == 0);
			assert!(tq <= 3);
			
			let slice = self.qtables.mut_slice(64 * tq as uint, 
											   64 * tq as uint + 64);
			let _ = try!(self.r.fill(slice));
									
			table_length -= 1 + 64;
		}
		
		Ok(())
	}

	fn read_huffman_tables(&mut self) -> IoResult<()> {
		let mut table_length = try!(self.r.read_be_u16());
		table_length -= 2;
		
		while table_length > 0 {
			let tcth = try!(self.r.read_u8());
			let tc = tcth >> 4;
			let th = tcth & 0x0F;
			
			assert!(tc == 0 || tc == 1);

			let bits = try!(self.r.read_exact(16));
			let len = bits.len();

			let mt = bits.iter().fold(0, |a, b| a + *b);			
			let huffval = try!(self.r.read_exact(mt as uint));

			if tc == 0 {
				self.dctables[th] = derive_tables(bits, huffval);
			}
			else {
				self.actables[th] = derive_tables(bits, huffval);
			}
			
			table_length -= 1 + len as u16 + mt as u16;
		}

		Ok(())
	}


	fn read_restart_interval(&mut self) -> IoResult<()> {
		let _length = try!(self.r.read_be_u16());
		self.interval = try!(self.r.read_be_u16());

		Ok(())
	}

	fn read_restart(&mut self) -> IoResult<()> {
		let w = (self.width + 7) / (self.mcu_h * 8) as u16;	
		let h = (self.height + 7) / (self.mcu_v * 8) as u16;		
		 		
		if self.interval != 0  && 
		   self.mcucount % self.interval == 0 && 
		   self.mcucount < w * h {
			
			let rst = try!(self.find_restart_marker());
			
			if rst == self.expected_rst {
				self.reset();
				self.expected_rst += 1;

				if self.expected_rst > RST7 {
					self.expected_rst = RST0;
				}
			}
			else {
				fail!(format!("expected marker {0:X} but got {1:X}", self.expected_rst, rst));
			}
		}

		Ok(())
	}

	fn find_restart_marker(&mut self) -> IoResult<u8> {
		if self.h.marker != 0 {
			let m = self.h.marker;
			self.h.marker = 0;
			
			return Ok(m);
		}
		
		let mut b;
		loop {
			b = try!(self.r.read_u8());

			if b == 0xFF {
				b = try!(self.r.read_u8());
				match b {
					RST0 .. RST7 => break,
					EOI => fail!("unexpected end of image"),
					_   => continue
				}
			}
		}

		Ok(b)
	}

	fn reset(&mut self) {
		self.h.bits = 0;
		self.h.num_bits = 0;
		self.h.end = false;
		self.h.marker = 0;

		for (_, c) in self.components.mut_iter() {
			c.dc_pred = 0;		
		}		
	}
}

fn upsample_mcu(out: &mut [u8], xoffset: uint, width: uint, bpp: uint, mcu: &[i32], h: u8, v: u8) {
	if mcu.len() == 64 {
		for y in range(0u, 8) {
			for x in range(0u, 8) {
				out[xoffset + x + (y * width)] = mcu[x + y * 8] as u8
			}
		} 
	}
	else {
		let y_blocks = h * v;

		let y_blocks = mcu.slice_to(y_blocks as uint * 64);
		let cb = mcu.slice(y_blocks.len(), y_blocks.len() + 64);
		let cr = mcu.slice_from(y_blocks.len() + cb.len());

		let mut k = 0;
		for by in range(0, v as uint) {
			let y0 = by * 8;

			for bx in range(0, h as uint) {
				let x0 = xoffset + bx * 8 * bpp;

				for y in range(0u, 8) {
					for x in range(0u, 8) {
						let (a, b, c) = (y_blocks[k * 64 + x + y * 8], cb[x + y * 8], cr[x + y * 8]);
						let (r, g, b) = ycbcr_to_rgb(a as f32, b as f32, c as f32);
						
						let offset = (y0 + y) * (width * bpp) + x0 + x * bpp;
						out[offset + 0] = r;
						out[offset + 1] = g;
						out[offset + 2] = b; 
					}
				}		

				k += 1;
			}
		}
	}
}

fn ycbcr_to_rgb(y: f32, cb: f32, cr: f32) -> (u8, u8, u8) {
	let r = y + 1.402f32 * (cr - 128f32) ;
	let g = y - 0.34414f32 * (cb - 128f32) - 0.71414f32 * (cr - 128f32);
	let b = y + 1.772f32 * (cb - 128f32);

	(r as u8, g as u8, b as u8)
}

fn level_shift(a: &mut [i32]) {
	for i in a.mut_iter() {
		if *i < -128 {
			*i = 0
		}
		else if *i > 127 {
			*i = 255
		}
		else {
			*i = *i + 128
		}
	}
}

//slow
fn slow_idct(s: &[i32]) -> ~[i32] {
	let a = 1.0 / f32::sqrt(2 as f32);
	let mut out = slice::from_elem(64, 0i32); 

	for y in range(0, 8) {
		for x in range(0, 8) {
			let mut sum = 0f32;

			for u in range(0, 8) {
				for v in range(0, 8) {
					let cu = if u == 0 {a} else {1f32};
					let cv = if v == 0 {a} else {1f32};
					
					let svu = s[v + 8 * u] as f32;
					
					let i = f32::cos(((2 * x + 1) as f32 * v as f32 * f32::consts::PI) / 16f32);
					let j = f32::cos(((2 * y + 1) as f32 * u as f32 * f32::consts::PI) / 16f32);
					
					sum += cu * cv * svu * i * j;
				}		
			}

			out[x + 8 * y] = f32::round(sum / 4f32) as i32;
		}
	}
	out
}

//Section F.2.2.1
//Figure F.12
fn extend(v: i32, t: u8) -> i32 {
	let vt = 1 << t as uint - 1;
	let vt = vt as i32;

	if v < vt {
		v + ((-1) << t as uint) + 1
	}
	else {
		v
	}
}

struct HuffDecoder {
	bits: u32,
	num_bits: u8,
	end: bool,
	marker: u8,
}

impl HuffDecoder {
	pub fn new() -> HuffDecoder {
		HuffDecoder {bits: 0, num_bits: 0, end: false, marker: 0}
	}
	
	fn guarantee<R: Reader>(&mut self, r: &mut R, n: u8) -> IoResult<()> {
		while self.num_bits < n && !self.end {
			let byte = try!(r.read_u8());

			if byte == 0xFF {
				let byte2 = try!(r.read_u8());
				if byte2 != 0 {
					self.marker = byte2;
					self.end = true;
				}
			}
			
			self.bits |= (byte as u32 << (32 - 8)) >> self.num_bits as u32;
			self.num_bits += 8;
		}
		
		Ok(())
	}

	pub fn read_bit<R: Reader>(&mut self, r: &mut R) -> IoResult<u8> {
		let _ = try!(self.guarantee(r, 1));

		let bit = (self.bits & (1 << 31)) >> 31;
		self.consume(1);

		Ok(bit as u8)
	}	
	
	//Section F.2.2.4
	//Figure F.17
	pub fn receive<R: Reader>(&mut self, r: &mut R, ssss: u8) -> IoResult<i32> {
		let _ = try!(self.guarantee(r, ssss));
		
		let bits = (self.bits & (0xFFFFFFFFu32 << (32 - ssss as u32))) >> (32 - ssss);
		self.consume(ssss);
		
		Ok(bits as i32)
	}

	fn consume(&mut self, n: u8) {
		self.bits <<= n as u32;
		self.num_bits -= n;
	}

	pub fn decode_symbol<R: Reader>(&mut self, r: &mut R, table: &HuffTable) -> IoResult<u8> {
		let _ = try!(self.guarantee(r, 8));
		let index = (self.bits & 0xFF000000) >> (32 - 8);
		let (val, size) = table.lut[index];

		if index < 256 && size < 9 {			
			self.consume(size);
				
			return Ok(val)
		}
		else {
			let mut code = 0u;
				
			for i in range(0, 16) {
				let b = try!(self.read_bit(r));
				code |= b as uint;

				if (code as int) <= table.maxcode[i] {
					let index = table.valptr[i] + code as int - table.mincode[i];
					return Ok(table.huffval[index])	
				}
				code <<= 1;
			}

			fail!(format!("bad huffman code: {:t}", code));
		}
	}	
}

fn derive_tables(bits: ~[u8], huffval: ~[u8]) -> HuffTable {
	let mut huffsize = slice::from_elem(256, 0u8);
	let mut huffcode = slice::from_elem(256, 0u16);
	let mut mincode  = slice::from_elem(16, -1i);
	let mut maxcode  = slice::from_elem(16, -1i);
	let mut valptr   = slice::from_elem(16, -1i);
	let mut lut 	 = slice::from_elem(256, (0u8, 17u8));
	
	let mut k = 0;
	let mut j;
	
	//Annex C.2
	//Figure C.1
	//Generate table of individual code lengths
	for i in range(0u, 16) {
		j = 0;
		while j < bits[i] { 
			huffsize[k] = i as u8 + 1;
			k += 1;
			j += 1;		
		}
	}

	huffsize[k] = 0;
	
	//Annex C.2
	//Figure C.2
	//Generate table of huffman codes
	k = 0;
	let mut code = 0u16;
	let mut size = huffsize[0];

	while huffsize[k] != 0 {
		huffcode[k] = code;
		code += 1;
		k += 1;

		if huffsize[k] == size {
			continue
		}

		let diff = huffsize[k] - size;
		code <<= diff as u16;

		size += diff
	}
	
	//Annex F.2.2.3
	//Figure F.15
	let mut j = 0;
	
	for i in range(0u, 16) {
		if bits[i] != 0 {
			valptr[i] = j;
			mincode[i] = huffcode[j] as int;
			j += bits[i] as int - 1;
			maxcode[i] = huffcode[j] as int;
			j += 1;
		}
	}
	
	for (i, v) in huffval.iter().enumerate() {
		if huffsize[i] > 8 {
			break
		}
		
		let r = 8 - huffsize[i] as uint;
		
		for j in range(0, 1 << r) {
			let index = (huffcode[i] << r) + j as u16;
			lut[index as uint] = (*v, huffsize[i]);
		}
	}

	HuffTable {
		lut: lut,
		huffval: huffval,		
		maxcode: maxcode,
		mincode: mincode,
		valptr: valptr
	}
}