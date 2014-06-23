//! Types and functions for working with pixels, where the colortype is not known
//! at compile time.

use pixel::{
        Pixel,
        Luma,
        LumaA,
        Rgb,
        Rgba
};

use colortype;
use colortype::ColorType;
use colorops;
use sample;

/// An abstraction over a vector of pixel types
#[deriving(Clone, Show, PartialEq, Eq)]
pub enum PixelBuf {
        /// Each pixel in this buffer is 8-bit Luma
        Luma8(Vec<Luma<u8>>),
        //Luma16(Vec<Luma<u16>>),

        /// Each pixel in this buffer is 8-bit Luma with alpha
        LumaA8(Vec<LumaA<u8>>),
        //LumaA16(Vec<LumaA<u16>>),

        /// Each pixel in this buffer is 8-bit Rgb
        Rgb8(Vec<Rgb<u8>>),
        //Rgb16(Vec<Rgb<u16>>),

        /// Each pixel in this buffer is 8-bit Rgb with alpha
        Rgba8(Vec<Rgba<u8>>),
        //Rgba16(Vec<Rgba<u16>>),
}

impl PixelBuf {
        /// Convert from self to an array of 8-bit Luma pixels.
        pub fn as_luma8<'a>(&'a self) -> &'a [Luma<u8>] {
                match *self {
                        Luma8(ref p) => p.as_slice(),
                        _            => &[]
                }
        }

        /// Convert from self to an array of 8-bit Luma pixels with alpha.
        pub fn as_luma_alpha8<'a>(&'a self) -> &'a [LumaA<u8>] {
                match *self {
                        LumaA8(ref p) => p.as_slice(),
                        _             => &[]
                }
        }

        /// Convert from self to an array of 8-bit RGB pixels.
        pub fn as_rgb8<'a>(&'a self) -> &'a [Rgb<u8>] {
                match *self {
                        Rgb8(ref p) => p.as_slice(),
                        _           => &[]
                }
        }

        /// Convert from self to an array of 8-bit RGB pixels with alpha.
        pub fn as_rgba8<'a>(&'a self) -> &'a [Rgba<u8>] {
                match *self {
                        Rgba8(ref p) => p.as_slice(),
                        _            => &[]
                }
        }

        /// Convert from a vector of bytes to a ```PixelBuf```
        /// Returns the original buffer if the conversion cannot be done.
        pub fn from_bytes(buf: Vec<u8>, color: ColorType) -> Result<PixelBuf, Vec<u8>> {
                //TODO: consider transmuting
                match color {
                        colortype::RGB(8) => {
                                let p = buf.as_slice()
                                           .chunks(3)
                                           .map(|a| Rgb::<u8>(a[0], a[1], a[2]))
                                           .collect();

                                Ok(Rgb8(p))
                        }

                        colortype::RGBA(8) => {
                                let p = buf.as_slice()
                                           .chunks(4)
                                           .map(|a| Rgba::<u8>(a[0], a[1], a[2], a[3]))
                                           .collect();

                                Ok(Rgba8(p))
                        }

                        colortype::Grey(8) => {
                                let p = buf.as_slice()
                                           .iter()
                                           .map(|a| Luma::<u8>(*a))
                                           .collect();

                                Ok(Luma8(p))
                        }

                        colortype::GreyA(8) => {
                                let p = buf.as_slice()
                                           .chunks(2)
                                           .map(|a| LumaA::<u8>(a[0], a[1]))
                                           .collect();

                                Ok(LumaA8(p))
                        }

                        _ => Err(buf)
                }
        }

        /// Convert from a ```PixelBuf``` to a vector of bytes
        pub fn to_bytes(&self) -> Vec<u8> {
                let mut r = Vec::new();
                //TODO: consider transmuting

                match *self {
                        Luma8(ref a) => {
                                for &i in a.iter() {
                                        r.push(i.channel());
                                }
                        }

                        LumaA8(ref a) => {
                                for &i in a.iter() {
                                        let (l, a) = i.channels();
                                        r.push(l);
                                        r.push(a);
                                }
                        }

                        Rgb8(ref a)  => {
                                for &i in a.iter() {
                                        let (red, g, b) = i.channels();
                                        r.push(red);
                                        r.push(g);
                                        r.push(b);
                                }
                        }

                        Rgba8(ref a) => {
                                for &i in a.iter() {
                                        let (red, g, b, alpha) = i.channels();
                                        r.push(red);
                                        r.push(g);
                                        r.push(b);
                                        r.push(alpha);
                                }
                        }
                }

                r
        }
}

/// Convert ```pixels``` Pixelbuf to grayscale
pub fn grayscale(pixels: &PixelBuf) -> PixelBuf {
        match *pixels {
                Luma8(ref p)  => Luma8(colorops::grayscale(p.as_slice())),
                LumaA8(ref p) => Luma8(colorops::grayscale(p.as_slice())),
                Rgb8(ref p)   => Luma8(colorops::grayscale(p.as_slice())),
                Rgba8(ref p)  => Luma8(colorops::grayscale(p.as_slice())),
        }
}

/// Invert the pixels in ```PixelBuf```
pub fn invert(pixels: &mut PixelBuf) {
        match *pixels {
                Luma8(ref mut p)  => colorops::invert(p.as_mut_slice()),
                LumaA8(ref mut p) => colorops::invert(p.as_mut_slice()),
                Rgb8(ref mut p)   => colorops::invert(p.as_mut_slice()),
                Rgba8(ref mut p)  => colorops::invert(p.as_mut_slice()),
        }
}

/// Resize this ```PixelBuf``` pixels.
/// ```width``` and ```height``` are the original dimensions.
/// ```nwidth``` and ```nheight``` are the new dimensions.
pub fn resize(pixels:  &PixelBuf,
              width:   u32,
              height:  u32,
              nwidth:  u32,
              nheight: u32,
              filter:  sample::FilterType) -> PixelBuf {

        match *pixels {
                Luma8(ref p)  => Luma8(sample::resize(p.as_slice(), width, height, nwidth, nheight, filter)),
                LumaA8(ref p) => LumaA8(sample::resize(p.as_slice(), width, height, nwidth, nheight, filter)),
                Rgb8(ref p)   => Rgb8(sample::resize(p.as_slice(), width, height, nwidth, nheight, filter)),
                Rgba8(ref p)  => Rgba8(sample::resize(p.as_slice(), width, height, nwidth, nheight, filter)),
        }
}

/// Perfomrs a Gausian blur on this ```Pixelbuf```.
/// ```width``` and ```height``` are the dimensions of the buffer.
/// ```sigma``` is a meausure of how much to blur by.
pub fn blur(pixels:  &PixelBuf,
            width:   u32,
            height:  u32,
            sigma:   f32) -> PixelBuf {

        match *pixels {
                Luma8(ref p)  => Luma8(sample::blur(p.as_slice(), width, height, sigma)),
                LumaA8(ref p) => LumaA8(sample::blur(p.as_slice(), width, height, sigma)),
                Rgb8(ref p)   => Rgb8(sample::blur(p.as_slice(), width, height, sigma)),
                Rgba8(ref p)  => Rgba8(sample::blur(p.as_slice(), width, height, sigma)),
        }
}

/// Performs an unsharpen mask on ```pixels```
/// ```sigma``` is the amount to blur the image by.
/// ```threshold``` is a control of how much to sharpen.
/// see https://en.wikipedia.org/wiki/Unsharp_masking#Digital_unsharp_masking
pub fn unsharpen(pixels:    &PixelBuf,
                 width:     u32,
                 height:    u32,
                 sigma:     f32,
                 threshold: i32) -> PixelBuf {

        match *pixels {
                Luma8(ref p)  => Luma8(sample::unsharpen(p.as_slice(), width, height, sigma, threshold)),
                LumaA8(ref p) => LumaA8(sample::unsharpen(p.as_slice(), width, height, sigma, threshold)),
                Rgb8(ref p)   => Rgb8(sample::unsharpen(p.as_slice(), width, height, sigma, threshold)),
                Rgba8(ref p)  => Rgba8(sample::unsharpen(p.as_slice(), width, height, sigma, threshold)),
        }
}

/// Filters the pixelbuf with the specified 3x3 kernel.
pub fn filter3x3(pixels:  &PixelBuf,
                 width:   u32,
                 height:  u32,
                 kernel:  &[f32]) -> PixelBuf {

        if kernel.len() != 9 {
                return pixels.clone()
        }

        match *pixels {
                Luma8(ref p)  => Luma8(sample::filter3x3(p.as_slice(), width, height, kernel)),
                LumaA8(ref p) => LumaA8(sample::filter3x3(p.as_slice(), width, height, kernel)),
                Rgb8(ref p)   => Rgb8(sample::filter3x3(p.as_slice(), width, height, kernel)),
                Rgba8(ref p)  => Rgba8(sample::filter3x3(p.as_slice(), width, height, kernel)),
        }
}

/// Adjust the contrast of ```pixels```
/// ```contrast``` is the amount to adjust the contrast by.
/// Negative values decrease the constrast and positive values increase the constrast.
pub fn adjust_contrast(pixels: &PixelBuf, c: f32) -> PixelBuf {
        match *pixels {
                Luma8(ref p)  => Luma8(colorops::contrast(p.as_slice(), c)),
                LumaA8(ref p) => LumaA8(colorops::contrast(p.as_slice(), c)),
                Rgb8(ref p)   => Rgb8(colorops::contrast(p.as_slice(), c)),
                Rgba8(ref p)  => Rgba8(colorops::contrast(p.as_slice(), c)),
        }
}

/// Brighten ```pixels```
/// ```value``` is the amount to brighten each pixel by.
/// Negative values decrease the brightness and positive values increase it.
pub fn brighten(pixels: &PixelBuf, c: i32) -> PixelBuf {
        match *pixels {
                Luma8(ref p)  => Luma8(colorops::brighten(p.as_slice(), c)),
                LumaA8(ref p) => LumaA8(colorops::brighten(p.as_slice(), c)),
                Rgb8(ref p)   => Rgb8(colorops::brighten(p.as_slice(), c)),
                Rgba8(ref p)  => Rgba8(colorops::brighten(p.as_slice(), c)),
        }
}