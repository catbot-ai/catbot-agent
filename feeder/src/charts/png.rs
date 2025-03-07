use std::ops::Deref;

use image::ImageEncoder;

use image::{codecs::png::PngEncoder, ImageBuffer, ImageError, Pixel};

pub fn encode_png<P, Container>(img: &ImageBuffer<P, Container>) -> Result<Vec<u8>, ImageError>
where
    P: Pixel<Subpixel = u8> + image::PixelWithColorType + 'static,
    Container: Deref<Target = [P::Subpixel]>,
{
    let mut buf = Vec::new();
    let encoder = PngEncoder::new(&mut buf);
    encoder.write_image(img, img.width(), img.height(), P::COLOR_TYPE)?;
    Ok(buf)
}
