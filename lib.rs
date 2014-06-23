//! This crate provides native rust implementations of
//! Image encoders and decoders and basic image manipulation
//! functions.

#![crate_id = "image"]
#![crate_type = "rlib"]

#![allow(missing_doc)]
#![feature(macro_rules)]

extern crate flate;

pub use ColorType = colortype::ColorType;
pub use Grey      = colortype::Grey;
pub use RGB       = colortype::RGB;
pub use Palette   = colortype::Palette;
pub use GreyA     = colortype::GreyA;
pub use RGBA      = colortype::RGBA;

pub use ImageDecoder = image::ImageDecoder;
pub use ImageError   = image::ImageError;
pub use ImageResult  = image::ImageResult;
pub use ImageFormat  = image::ImageFormat;
pub use FilterType   = sample::FilterType;

pub use sample::{
        Triangle,
        Nearest,
        CatmullRom,
        Gaussian,
        Lanczos3
};

pub use image::{
        PNG,
        JPEG,
        GIF,
        WEBP,
        PPM
};

pub use Image = image::Image;

pub use JPEGDecoder = jpeg::JPEGDecoder;
pub use JPEGEncoder = jpeg::JPEGEncoder;
pub use PNGDecoder  = png::PNGDecoder;
pub use PNGEncoder  = png::PNGEncoder;
pub use GIFDecoder  = gif::GIFDecoder;
pub use PPMEncoder  = ppm::PPMEncoder;
pub use WebpDecoder = webp::WebpDecoder;

//Codecs
#[path = "codecs/vp8.rs"]
pub mod vp8;

#[path = "codecs/jpeg/mod.rs"]
pub mod jpeg;

#[path = "codecs/png/mod.rs"]
pub mod png;

#[path = "codecs/gif/mod.rs"]
pub mod gif;

#[path = "codecs/webp/mod.rs"]
pub mod webp;

#[path = "codecs/ppm.rs"]
pub mod ppm;

#[path = "codecs/hash.rs"]
mod hash;

#[path = "codecs/transform.rs"]
mod transform;

#[path = "codecs/deflate.rs"]
mod deflate;

#[path = "codecs/zlib.rs"]
mod zlib;

#[path = "codecs/lzw.rs"]
mod lzw;

//Imaging
#[path = "imaging/colortype.rs"]
pub mod colortype;

#[path = "imaging/pixel.rs"]
pub mod pixel;

#[path = "imaging/sample.rs"]
pub mod sample;

#[path = "imaging/colorops.rs"]
pub mod colorops;

#[path = "imaging/pixelbuf.rs"]
pub mod pixelbuf;

mod image;