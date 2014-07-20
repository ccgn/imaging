use std::{
	io,
	slice
};

use std::ascii::StrAsciiExt;
use std::default::Default;
use color;

use color::{
        Pixel,
        ColorType
};

use ppm;
use gif;
use webp;
use jpeg;
use png;

/// An enumeration of Image Errors
#[deriving(Show, PartialEq, Eq)]
pub enum ImageError {
        ///The Image is not formatted properly
	FormatError,

        ///The Image's dimensions are either too small or too large
	DimensionError,

        ///The Decoder does not support this image format
	UnsupportedError,

        ///The Decoder does not support this color type
	UnsupportedColor,

        ///Not enough data was provided to the Decoder
        ///to decode the image
	NotEnoughData,

        ///An I/O Error occurred while decoding the image
	IoError,

        ///The end of the image has been reached
        ImageEnd
}

pub type ImageResult<T> = Result<T, ImageError>;

/// An enumeration of supported image formats.
/// Not all formats support both encoding and decoding.
#[deriving(PartialEq, Eq, Show)]
pub enum ImageFormat {
	/// An Image in PNG Format
	PNG,

	/// An Image in JPEG Format
	JPEG,

	/// An Image in GIF Format
	GIF,

	/// An Image in WEBP Format
	WEBP,

	/// An Image in PPM Format
	PPM
}

/// The trait that all decoders implement
pub trait ImageDecoder {
        ///Return a tuple containing the width and height of the image
        fn dimensions(&mut self) -> ImageResult<(u32, u32)>;

        ///Return the color type of the image e.g RGB(8) (8bit RGB)
        fn colortype(&mut self) -> ImageResult<ColorType>;

        ///Returns the length in bytes of one decoded row of the image
        fn row_len(&mut self) -> ImageResult<uint>;

        ///Read one row from the image into buf
        ///Returns the row index
        fn read_scanline(&mut self, buf: &mut [u8]) -> ImageResult<u32>;

        ///Decode the entire image and return it as a Vector
        fn read_image(&mut self) -> ImageResult<Vec<u8>>;

        ///Decode a specific region of the image, represented by the rectangle
        ///starting from ```x``` and ```y``` and having ```length``` and ```width```
        fn load_rect(&mut self, x: u32, y: u32, length: u32, width: u32) -> ImageResult<Vec<u8>> {
                let (w, h) = try!(self.dimensions());

                if length > h || width > w || x > w || y > h {
                        return Err(DimensionError)
                }

                let c = try!(self.colortype());

                let bpp = color::bits_per_pixel(c) / 8;
                let rowlen  = try!(self.row_len());

                let mut buf = Vec::from_elem(length as uint * width as uint * bpp, 0u8);
                let mut tmp = Vec::from_elem(rowlen, 0u8);

                loop {
                        let row = try!(self.read_scanline(tmp.as_mut_slice()));
                        if row - 1 == y {
                                break
                        }
                }

                for i in range(0, length as uint) {
                        {
                                let from = tmp.slice_from(x as uint * bpp)
                                              .slice_to(width as uint * bpp);

                                let to   = buf.mut_slice_from(i * width as uint * bpp)
                                              .mut_slice_to(width as uint * bpp);

                                slice::bytes::copy_memory(to, from);
                        }

                        let _ = try!(self.read_scanline(tmp.as_mut_slice()));
                }

                Ok(buf)
        }
}

/// Immutable pixel iterator
pub struct Pixels<'a, I> {
        image:  &'a I,
        x:      u32,
        y:      u32,
        width:  u32,
        height: u32
}

impl<'a, T: Primitive, P: Pixel<T>, I: GenericImage<P>> Iterator<(u32, u32, P)> for Pixels<'a, I> {
        fn next(&mut self) -> Option<(u32, u32, P)> {
                if self.x >= self.width {
                        self.x =  0;
                        self.y += 1;
                }

                if self.y >= self.height {
                        None
                } else {
                        let pixel = self.image.get_pixel(self.x, self.y);
                        let p = (self.x, self.y, pixel);

                        self.x += 1;

                        Some(p)
                }
        }
}

///A trait for manipulating images.
pub trait GenericImage<P> {
        ///The width and height of this image.
        fn dimensions(&self) -> (u32, u32);

        ///The bounding rectange of this image.
        fn bounds(&self) -> (u32, u32, u32, u32);

        ///Return the pixel located at (x, y)
        fn get_pixel(&self, x: u32, y: u32) -> P;

        ///Put a pixel at location (x, y)
        fn put_pixel(&mut self, x: u32, y: u32, pixel: P);

        ///Return an Iterator over the pixels of this image
        fn pixels<'a>(&'a self) -> Pixels<'a, Self> {
                let (width, height) = self.dimensions();

                Pixels {
                        image:  self,
                        x:      0,
                        y:      0,
                        width:  width,
                        height: height,
                }
        }
}

///An Image whose pixels are contained within a vector
#[deriving(Clone)]
pub struct ImageBuf<P> {
	pixels:  Vec<P>,
	width:   u32,
	height:  u32,
}

impl<T: Primitive, P: Pixel<T>> ImageBuf<P> {
        ///Construct a new ImageBuf with the specified with and height.
        pub fn new(width: u32, height: u32) -> ImageBuf<P> {
                let pixel: P = Default::default();
                let pixels = Vec::from_elem((width * height) as uint, pixel.clone());

                ImageBuf {
                        pixels:  pixels,
                        width:   width,
                        height:  height,
                }
        }

	///Construct a new ImageBuf from a vector of pixels.
	pub fn from_pixels(pixels: Vec<P>, width: u32, height: u32) -> ImageBuf<P> {
		ImageBuf {
			pixels:  pixels,
			width:   width,
			height:  height,
		}
	}

        ///Construct a new ImageBuf from a pixel.
        pub fn from_pixel(width: u32, height: u32, pixel: P) -> ImageBuf<P> {
                let buf = Vec::from_elem(width as uint * height as uint, pixel.clone());

                ImageBuf::from_pixels(buf, width, height)
        }

        ///An iterator over the pixels of this ImageBuf
        pub fn iter<'a>(&'a self) -> slice::Items<'a, P> {
                self.iter()
        }
}

impl<T: Primitive, P: Pixel<T> + Clone + Copy> GenericImage<P> for ImageBuf<P> {
        fn dimensions(&self) -> (u32, u32) {
                (self.width, self.height)
        }

        fn bounds(&self) -> (u32, u32, u32, u32) {
                (0, 0, self.width, self.height)
        }

        fn get_pixel(&self, x: u32, y: u32) -> P {
                let index  = y * self.width + x;
                let buf    = self.pixels.as_slice();

                buf[index as uint]
        }

        fn put_pixel(&mut self, x: u32, y: u32, pixel: P) {
                let index  = y * self.width + x;
                let buf    = self.pixels.as_mut_slice();

                buf[index as uint] = pixel;
        }
}

/// A View into another image
pub struct SubImage<'a, I> {
        image:   &'a mut I,
        xoffset: u32,
        yoffset: u32,
        xstride: u32,
        ystride: u32,
}

impl<'a, T: Primitive, P: Pixel<T>, I: GenericImage<P>> SubImage<'a, I> {
        ///Construct a new subimage
        pub fn new(image: &'a mut I, x: u32, y: u32, width: u32, height: u32) -> SubImage<'a, I> {
                SubImage {
                        image:   image,
                        xoffset: x,
                        yoffset: y,
                        xstride: width,
                        ystride: height,
                }
        }

        ///Convert this subimage to an ImageBuf
        pub fn to_image(&self) -> ImageBuf<P> {
                let p: P = Default::default();
                let mut out = ImageBuf::from_pixel(self.xstride, self.ystride, p.clone());

                for y in range(0, self.ystride) {
                        for x in range(0, self.xstride) {
                                let p = self.get_pixel(x, y);
                                out.put_pixel(x, y, p);
                        }
                }

                out
        }
}

impl<'a, T: Primitive, P: Pixel<T>, I: GenericImage<P>> GenericImage<P> for SubImage<'a, I> {
        fn dimensions(&self) -> (u32, u32) {
                (self.xstride, self.ystride)
        }

        fn bounds(&self) -> (u32, u32, u32, u32) {
                (self.xoffset, self.yoffset, self.xstride, self.ystride)
        }

        fn get_pixel(&self, x: u32, y: u32) -> P {
                self.image.get_pixel(x + self.xoffset, y + self.yoffset)
        }

        fn put_pixel(&mut self, x: u32, y: u32, pixel: P) {
                self.image.put_pixel(x + self.xoffset, y + self.yoffset, pixel)
        }
}

///A Dynamic Image
#[deriving(Clone)]
pub enum DynamicImage {
        /// Each pixel in this image is 8-bit Luma
        ImageLuma8(ImageBuf<color::Luma<u8>>),

        /// Each pixel in this image is 8-bit Luma with alpha
        ImageLumaA8(ImageBuf<color::LumaA<u8>>),

        /// Each pixel in this image is 8-bit Rgb
        ImageRgb8(ImageBuf<color::Rgb<u8>>),

        /// Each pixel in this image is 8-bit Rgb with alpha
        ImageRgba8(ImageBuf<color::Rgba<u8>>),
}

impl DynamicImage {
        ///Return the width and height of this image.
        pub fn dimensions(&self) -> (u32, u32) {
                match *self {
                        ImageLuma8(ref a) => a.dimensions(),
                        ImageLumaA8(ref a) => a.dimensions(),
                        ImageRgb8(ref a) => a.dimensions(),
                        ImageRgba8(ref a) => a.dimensions(),
                }
        }

        ///Return this image's pixels as a byte vector.
        pub fn raw_pixels(&self) -> Vec<u8> {
                image_to_bytes(self)
        }

        ///Return this image's color type.
        pub fn color(&self) -> ColorType {
                match *self {
                        ImageLuma8(_) => color::Grey(8),
                        ImageLumaA8(_) => color::GreyA(8),
                        ImageRgb8(_) => color::RGB(8),
                        ImageRgba8(_) => color::RGBA(8),
                }
        }

        /// Encode this image and write it to ```w```
        pub fn save<W: Writer>(&self, w: W, format: ImageFormat) -> io::IoResult<ImageResult<()>> {
                let bytes = self.raw_pixels();
                let (width, height) = self.dimensions();
                let color = self.color();

                let r = match format {
                        PNG  => {
                                let mut p = png::PNGEncoder::new(w);
                                try!(p.encode(bytes.as_slice(), width, height, color))
                                Ok(())
                        }

                        PPM  => {
                                let mut p = ppm::PPMEncoder::new(w);
                                try!(p.encode(bytes.as_slice(), width, height, color))
                                Ok(())
                        }

                        JPEG => {
                                let mut j = jpeg::JPEGEncoder::new(w);
                                try!(j.encode(bytes.as_slice(), width, height, color))
                                Ok(())
                        }

                        _    => Err(UnsupportedError),
                };

                Ok(r)
        }
}

fn decoder_to_image<I: ImageDecoder>(codec: I) -> ImageResult<DynamicImage> {
	let mut codec = codec;

	let color  = try!(codec.colortype());
	let buf    = try!(codec.read_image());
	let (w, h) = try!(codec.dimensions());

	let image = match color {
		color::RGB(8) => {
                        let p = buf.as_slice()
                                   .chunks(3)
                                   .map(|a| color::Rgb::<u8>(a[0], a[1], a[2]))
                                   .collect();

                        ImageRgb8(ImageBuf::from_pixels(p, w, h))
                }

                color::RGBA(8) => {
                        let p = buf.as_slice()
                                   .chunks(4)
                                   .map(|a| color::Rgba::<u8>(a[0], a[1], a[2], a[3]))
                                   .collect();

                        ImageRgba8(ImageBuf::from_pixels(p, w, h))
                }

                color::Grey(8) => {
                        let p = buf.as_slice()
                                   .iter()
                                   .map(|a| color::Luma::<u8>(*a))
                                   .collect();

                        ImageLuma8(ImageBuf::from_pixels(p, w, h))
                }

                color::GreyA(8) => {
                        let p = buf.as_slice()
                                   .chunks(2)
                                   .map(|a| color::LumaA::<u8>(a[0], a[1]))
                                   .collect();

                        ImageLumaA8(ImageBuf::from_pixels(p, w, h))
                }

                _ => return Err(UnsupportedColor)
	};

	Ok(image)
}

fn image_to_bytes(image: &DynamicImage) -> Vec<u8> {
        let mut r = Vec::new();

        match *image {
                //TODO: consider transmuting
                ImageLuma8(ref a) => {
                        for &i in a.iter() {
                                r.push(i.channel());
                        }
                }

                ImageLumaA8(ref a) => {
                        for &i in a.iter() {
                                let (l, a) = i.channels();
                                r.push(l);
                                r.push(a);
                        }
                }

                ImageRgb8(ref a)  => {
                        for &i in a.iter() {
                                let (red, g, b) = i.channels();
                                r.push(red);
                                r.push(g);
                                r.push(b);
                        }
                }

                ImageRgba8(ref a) => {
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

/// Open the image located at the path specified.
/// The image's format is determined from the path's file extension.
pub fn open(path: &Path) -> ImageResult<DynamicImage> {
        let fin = match io::File::open(path) {
                Ok(f)  => f,
                Err(_) => return Err(IoError)
        };

        let ext    = path.extension_str()
                         .map_or("".to_string(), |s| s.to_ascii_lower());

        let format = match ext.as_slice() {
                "jpg" |
                "jpeg" => JPEG,
                "png"  => PNG,
                "gif"  => GIF,
                "webp" => WEBP,
                _      => return Err(UnsupportedError)
        };

        load(fin, format)
}

/// Create a new image from a Reader
pub fn load<R: Reader>(r: R, format: ImageFormat) -> ImageResult<DynamicImage> {
        match format {
                PNG  => decoder_to_image(png::PNGDecoder::new(r)),
                GIF  => decoder_to_image(gif::GIFDecoder::new(r)),
                JPEG => decoder_to_image(jpeg::JPEGDecoder::new(r)),
                WEBP => decoder_to_image(webp::WebpDecoder::new(r)),
                _    => Err(UnsupportedError),
        }
}

/// Create a new image from a byte slice
pub fn load_from_memory(buf: &[u8], format: ImageFormat) -> ImageResult<DynamicImage> {
        let b = io::BufReader::new(buf);

        load(b, format)
}