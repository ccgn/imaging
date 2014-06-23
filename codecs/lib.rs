#![crate_id = "imagecodec"]
#![crate_type = "rlib"]

#![allow(missing_doc)]
#![feature(macro_rules)]

extern crate flate;

pub use ColorType = colortype::ColorType;
pub use Grey = colortype::Grey;
pub use RGB = colortype::RGB;
pub use Palette = colortype::Palette;
pub use GreyA = colortype::GreyA;
pub use RGBA = colortype::RGBA;

pub use ImageDecoder = codec::ImageDecoder;

pub use codec::{
        PNG,
        JPEG,
        GIF,
        WEBP,
        PPM
};

pub use JPEGDecoder = jpeg::JPEGDecoder;
pub use JPEGEncoder = jpeg::JPEGEncoder;
pub use PNGDecoder  = png::PNGDecoder;
pub use PNGEncoder  = png::PNGEncoder;
pub use GIFDecoder  = gif::GIFDecoder;
pub use PPMEncoder  = ppm::PPMEncoder;
pub use WebpDecoder = webp::WebpDecoder;

pub mod colortype;
pub mod vp8;
pub mod jpeg;
pub mod png;
pub mod gif;
pub mod webp;
pub mod ppm;

mod hash;
mod transform;
mod deflate;
mod zlib;
mod lzw;