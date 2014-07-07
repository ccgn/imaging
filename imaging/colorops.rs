//! Functions for altering and converting the color of pixelbufs
use std::num::{
        cast,
        Bounded
};

use imaging::pixel::{
        Pixel,
        Luma
};

use std::default::Default;
use image::GenericImage;
use image::ImageBuf;

fn clamp<N: Num + PartialOrd>(a: N, min: N, max: N) -> N {
        if a > max { max }
        else if a < min { min }
        else { a }
}

/// Convert ```pixels``` to grayscale
pub fn grayscale<P: Primitive + Default, T: Pixel<P> + Default + Copy + Clone, I: GenericImage<T>>(
        image: &I) -> ImageBuf<Luma<P>> {

        let (width, height) = image.dimensions();

        let d: Luma<P> = Default::default();
        let mut out = ImageBuf::from_pixel(width, height, d);

        for y in range(0, height) {
                for x in range(0, width) {
                        let p = image.get_pixel(x, y).to_luma();
                        out.put_pixel(x, y, p);
                }
        }

        out
}

/// Invert each pixel within ```pixels```
/// This function operates in place.
pub fn invert<P: Primitive, T: Pixel<P> + Default + Copy + Clone, I: GenericImage<T>>(
        image: &mut I) {

        let (width, height) = image.dimensions();

        for y in range(0, height) {
                for x in range(0, width) {
                        let mut p = image.get_pixel(x, y);
                        p.invert();

                        image.put_pixel(x, y, p);
                }
        }
}

/// Adjust the contrast of ```pixels```
/// ```contrast``` is the amount to adjust the contrast by.
/// Negative values decrease the constrast and positive values increase the constrast.
pub fn contrast<P: Primitive, T: Pixel<P> + Default + Copy + Clone, I: GenericImage<T>>(
        image:    &I,
        contrast: f32) -> ImageBuf<T> {

        let (width, height) = image.dimensions();

        let d: T = Default::default();
        let mut out = ImageBuf::from_pixel(width, height, d);

        let max: P = Bounded::max_value();
        let max = cast::<P, f32>(max).unwrap();

        let percent = ((100.0 + contrast) / 100.0).powi(2);

        for y in range(0, height) {
                for x in range(0, width) {
                        let f = image.get_pixel(x, y).map(|b| {
                                let c = cast::<P, f32>(b).unwrap();
                                let d = ((c / max - 0.5) * percent  + 0.5) * max;
                                let e = clamp(d, 0.0, max);

                                cast::<f32, P>(e).unwrap()
                        });

                        out.put_pixel(x, y, f);
                }
        }

        out
}

/// Brighten ```pixels```
/// ```value``` is the amount to brighten each pixel by.
/// Negative values decrease the brightness and positive values increase it.
pub fn brighten<P: Primitive, T: Pixel<P> + Default + Copy + Clone, I: GenericImage<T>>(
        image: &I,
        value: i32) -> ImageBuf<T> {

        let (width, height) = image.dimensions();

        let d: T = Default::default();
        let mut out = ImageBuf::from_pixel(width, height, d);

        let max: P = Bounded::max_value();
        let max = cast::<P, i32>(max).unwrap();

        for y in range(0, height) {
                for x in range(0, width) {
                        let e = image.get_pixel(x, y).map(|b| {
                                let c = cast::<P, i32>(b).unwrap();
                                let d = clamp(c + value, 0, max);

                                cast::<i32, P>(d).unwrap()
                        });

                        out.put_pixel(x, y, e);
                }
        }

        out
}