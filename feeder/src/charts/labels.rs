use super::candle::{Chart, LabelStyle};

use ab_glyph::ScaleFont;
use ab_glyph::{Font, PxScale};

use super::constants::*;
use image::{ImageBuffer, Rgb};
use imageproc::drawing::{draw_filled_rect_mut, draw_hollow_rect_mut, draw_text_mut, text_size};
use imageproc::rect::Rect;
use std::error::Error;

pub fn draw_labels(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    font: &impl Font,
    chart: &Chart,
    final_width: u32,
    height: u32,
) -> Result<(), Box<dyn Error>> {
    let white = Rgb([255u8, 255u8, 255u8]);

    if !chart.labels.is_empty() {
        let style = chart.label_style.clone().unwrap_or(LabelStyle {
            scale: PxScale { x: 15.0, y: 15.0 },
            color: white,
            background_color: TRANSPARENT_BLACK_50,
            offset_x: 5,
            offset_y: 0,
        });

        for (x, y, text) in chart.labels.iter() {
            let x_pos = (*x * final_width as f32) + style.offset_x as f32;
            let y_pos = (*y * height as f32) + style.offset_y as f32;
            draw_label(
                img,
                font,
                text,
                x_pos,
                y_pos,
                style.scale,
                style.color,
                Some(style.background_color),
            )?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments, unused)]
pub fn draw_label<F: Font>(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    font: &F,
    text: &str,
    x: f32,
    y: f32,
    scale: PxScale,
    color: Rgb<u8>,
    background_color: Option<Rgb<u8>>,
) -> anyhow::Result<Rect> {
    let font_metrics = font.as_scaled(scale);
    let (text_width, text_height) = text_size(scale, font, text);
    let padding = 2f32 * scale.x / text_height as f32;
    let bounding_rect = Rect::at(x as i32, y as i32).of_size(
        text_width + 2 * padding as u32,
        text_height + 2 * padding as u32,
    );

    if let Some(background_color) = background_color {
        draw_filled_rect_mut(img, bounding_rect, background_color);
    };

    draw_text_mut(
        img,
        color,
        (x + padding) as i32,
        (y + padding + font_metrics.descent() / text_height as f32 * scale.y * 0.6) as i32,
        scale,
        font,
        text,
    );

    Ok(bounding_rect)
}

#[allow(clippy::too_many_arguments, unused)]
pub fn draw_hallow_label<F: Font>(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    font: &F,
    text: &str,
    x: f32,
    y: f32,
    scale: PxScale,
    font_color: Rgb<u8>,
    border_color: Rgb<u8>,
) -> anyhow::Result<Rect> {
    let font_metrics = font.as_scaled(scale);
    let (text_width, text_height) = text_size(scale, font, text);
    let padding = 2f32 * scale.x / text_height as f32;

    let rect = Rect::at(x as i32, y as i32).of_size(
        text_width + 2 * padding as u32,
        text_height + 2 * padding as u32,
    );
    draw_hollow_rect_mut(img, rect, border_color);

    draw_text_mut(
        img,
        font_color,
        (x + padding) as i32,
        (y + padding + font_metrics.descent() / text_height as f32 * scale.y * 0.6) as i32,
        scale,
        font,
        text,
    );

    Ok(rect)
}
