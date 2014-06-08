//! Types and methods for representing and manipulating colors
use std::num::Bounded;

///An enumeration over supported color types and their bit depths
#[deriving(PartialEq, Show, Clone)]
pub enum ColorType {
	///Pixel is greyscale
	Grey(u8),

	///Pixel contains R, G and B channels
	Rgb(u8),

	///Pixel is an index into a color palette
	Palette(u8),

	///Pixel is greyscale with an alpha channel
	GreyA(u8),

	///Pixel is RGB with an alpha channel
	Rgba(u8)
}

///Returns the number of bits contained in a pixel of ColorType c
pub fn bits_per_pixel(c: ColorType) -> uint {
	match c {
		Grey(n)    => n as uint,
		Rgb(n)     => 3 * n as uint,
		Palette(n) => 3 * n as uint,
		GreyA(n)   => 2 * n as uint,
		Rgba(n)    => 4 * n as uint,
	}
}

///Returns the number of color channels that make up this pixel
pub fn num_components(c: ColorType) -> uint {
	match c {
		Grey(_)    => 1,
		Rgb(_)     => 3,
		Palette(_) => 3,
		GreyA(_)   => 2,
		Rgba(_)    => 4,
	}
}

#[packed]
#[deriving(PartialEq, Clone, Show)]
pub struct Luma<T>(pub T);

impl<T: Primitive + NumCast + Clone + Bounded> Luma<T> {
	pub fn channel(&self) -> T {
		match *self {
			Luma(l) => l
		}
	}
}

#[packed]
#[deriving(PartialEq, Clone, Show)]
pub struct LumaA<T>(pub T, pub T);

impl<T: Primitive + NumCast + Clone + Bounded> LumaA<T> {
	pub fn channels(&self) -> (T, T) {
		match *self {
			LumaA(l, a) => (l, a)
		}
	}

	pub fn alpha(&self) -> T {
		match *self {
			LumaA(_, a) => a
		}
	}
}

#[packed]
#[deriving(PartialEq, Clone, Show)]
pub struct RGB<T>(pub T, pub T, pub T);

impl<T: Primitive + NumCast + Clone + Bounded> RGB<T> {
	pub fn channels(&self) -> (T, T, T) {
		match *self {
			RGB(r, g, b) => (r, g, b)
		}
	}
}

#[packed]
#[deriving(PartialEq, Clone, Show)]
pub struct RGBA<T>(pub T, pub T, pub T, pub T);

impl<T: Primitive + NumCast + Clone + Bounded> RGBA<T> {
	pub fn channels(&self) -> (T, T, T, T) {
		match *self {
			RGBA(r, g, b, a) => (r, g, b, a)
		}
	}

	pub fn alpha(&self) -> T {
		match *self {
			RGBA(_, _, _, a) => a
		}
	}
}

pub trait ConvertColor<T> {
	fn to_rgb(&self) -> RGB<T>;
	fn to_rgba(&self) -> RGBA<T>;
	fn to_luma(&self) -> Luma<T>;
	fn to_luma_alpha(&self) -> LumaA<T>;
	fn invert(&mut self);
}

impl<T: Primitive + NumCast + Clone + Bounded> ConvertColor<T> for RGB<T> {
	fn to_luma(&self) -> Luma<T> {
		let (r, g, b) = self.channels();

		let l = 0.2125f32 * r.to_f32().unwrap() +
			0.7154f32 * g.to_f32().unwrap() +
			0.0721f32 * b.to_f32().unwrap();

		Luma(NumCast::from(l).unwrap())
	}

	fn to_luma_alpha(&self) -> LumaA<T> {
		let l = self.to_luma().channel();

		LumaA(l, Bounded::max_value())
	}

	fn to_rgb(&self) -> RGB<T> {
		self.clone()
	}

	fn to_rgba(&self) -> RGBA<T> {
		let (r, g, b) = self.channels();

		RGBA(r, g, b, Bounded::max_value())
	}

	fn invert(&mut self) {
		let (r, g, b) = self.channels();

		let max: T = Bounded::max_value();

		let r1 = max - r;
		let g1 = max - g;
		let b1 = max - b;

		*self = RGB(r1, g1, b1)
	}
}

impl<T: Primitive + NumCast + Clone + Bounded> ConvertColor<T> for RGBA<T> {
	fn to_luma(&self) -> Luma<T> {
		self.to_rgb().to_luma()
	}

	fn to_luma_alpha(&self) -> LumaA<T> {
		let l = self.to_luma().channel();
		let a = self.alpha();

		LumaA(l, a)
	}

	fn to_rgb(&self) -> RGB<T> {
		let (r, g, b, _) = self.channels();

		RGB(r, g, b)
	}

	fn to_rgba(&self) -> RGBA<T> {
		self.clone()
	}

	fn invert(&mut self) {
		let (r, g, b) = self.to_rgb().channels();
		let a = self.alpha();

		let max: T = Bounded::max_value();

		*self = RGBA(max - r, max - g, max - b, a)
	}
}

impl<T: Primitive + NumCast + Clone + Bounded> ConvertColor<T> for Luma<T> {
	fn to_luma(&self) -> Luma<T> {
		self.clone()
	}

	fn to_luma_alpha(&self) -> LumaA<T> {
		let l = self.channel();

		LumaA(l, Bounded::max_value())
	}

	fn to_rgb(&self) -> RGB<T> {
		let l1 = self.channel();
		let l2 = self.channel();
		let l3 = self.channel();

		RGB(l1, l2, l3)
	}

	fn to_rgba(&self) -> RGBA<T> {
		let (r, g, b) = self.to_rgb().channels();

		RGBA(r, g, b, Bounded::max_value())
	}

	fn invert(&mut self) {
		let max: T = Bounded::max_value();
		let l1 = max - self.channel();

		*self = Luma(l1)
	}
}

impl<T: Primitive + NumCast + Clone + Bounded> ConvertColor<T> for LumaA<T> {
	fn to_luma(&self) -> Luma<T> {
		let (l, _) = self.channels();
		Luma(l)
	}

	fn to_luma_alpha(&self) -> LumaA<T> {
		self.clone()
	}

	fn to_rgb(&self) -> RGB<T> {
		let (l1, _) = self.channels();
		let (l2, _) = self.channels();
		let (l3, _) = self.channels();

		RGB(l1, l2, l3)
	}

	fn to_rgba(&self) -> RGBA<T> {
		let (r, g, b) = self.to_rgb().channels();
		let a = self.alpha();

		RGBA(r, g, b, a)
	}

	fn invert(&mut self) {
		let l = self.to_luma().channel();
		let a  = self.alpha();

		let max: T = Bounded::max_value();

		*self = LumaA(max - l, a)
	}
}

pub fn rgb_to_ycbcr<C: Primitive + NumCast>(r: C, g: C, b: C) -> (C, C, C) {
	let r = r.to_f32().unwrap();
	let g = g.to_f32().unwrap();
	let b = b.to_f32().unwrap();

	let y  =  0.299f32  * r + 0.587f32  * g + 0.114f32  * b;
	let cb = -0.1687f32 * r - 0.3313f32 * g + 0.5f32    * b + 128f32;
	let cr =  0.5f32    * r - 0.4187f32 * g - 0.0813f32 * b + 128f32;

	(NumCast::from(y).unwrap(),
	 NumCast::from(cb).unwrap(),
	 NumCast::from(cr).unwrap())
}

pub fn ycbcr_to_rgb<C: Primitive + NumCast>(y: C, cb: C, cr: C) -> (C, C, C) {
	let y  =  y.to_f32().unwrap();
	let cr = cr.to_f32().unwrap();
	let cb = cb.to_f32().unwrap();

	let r1 = y + 1.402f32   * (cr - 128f32) ;
	let g1 = y - 0.34414f32 * (cb - 128f32) - 0.71414f32 * (cr - 128f32);
	let b1 = y + 1.772f32   * (cb - 128f32);

	(NumCast::from(r1).unwrap(),
	 NumCast::from(g1).unwrap(),
	 NumCast::from(b1).unwrap())
}