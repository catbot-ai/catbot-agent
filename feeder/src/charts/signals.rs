use super::image::draw_dashed_line_segment_mut;
use super::labels::{draw_hallow_label, draw_label};
use ab_glyph::Font;

use common::LongShortSignal;
use image::{ImageBuffer, Rgb};
use imageproc::drawing::draw_line_segment_mut;
use imageproc::rect::Rect;

use super::constants::*;
pub use plotters::style::full_palette::{BLACK, GREEN, RED};
use std::error::Error;

pub fn draw_signals(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    font: &impl Font,
    signals: &[LongShortSignal],
    current_price: f64,
    price_bounding_rect: Rect,
) -> Result<(), Box<dyn Error>> {
    signals.iter().for_each(|signal| {
        let x = price_bounding_rect.left() as f32;
        let mut y = price_bounding_rect.top() as f32 + (AXIS_SCALE.y - ORDER_LABEL_SCALE.y) / 2.0;
        let h = price_bounding_rect.height() as f32;

        // Too high?
        if price_bounding_rect.top() < 150 {
            y += 150.0;
        }

        let target_percent =
            ((signal.predicted.target_price - current_price) / current_price) * 100.0;
        let stop_percent = ((signal.predicted.stop_loss - current_price) / current_price) * 100.0;

        // Mark position
        let stop_y = if signal.predicted.direction == "long" {
            y - 2.0 * h
        } else {
            y + 2.0 * h
        };
        let stop_percent_y = if signal.predicted.direction == "long" {
            y - 3.0 * h
        } else {
            y + 3.0 * h
        };
        let entry_y = if signal.predicted.direction == "long" {
            y - 4.0 * h
        } else {
            y + 4.0 * h
        };
        let target_percent_y = if signal.predicted.direction == "long" {
            y - 5.0 * h
        } else {
            y + 5.0 * h
        };
        let target_y = if signal.predicted.direction == "long" {
            y - 6.0 * h
        } else {
            y + 6.0 * h
        };

        // Draw line
        let line_color = if signal.predicted.direction == "long" {
            Rgb([GREEN.0, GREEN.1, GREEN.2])
        } else {
            Rgb([RED.0, RED.1, RED.2])
        };

        draw_line_segment_mut(img, (x, entry_y), (x, target_y), line_color);

        draw_dashed_line_segment_mut(img, (x, stop_y), (x, entry_y), 4.0, 4.0, line_color);

        // Draw predicted price label
        let label_scale = ORDER_LABEL_SCALE;

        let color = if signal.predicted.direction == "long" {
            Rgb([GREEN.0, GREEN.1, GREEN.2])
        } else {
            Rgb([RED.0, RED.1, RED.2])
        };

        // stop
        let _ = draw_hallow_label(
            img,
            font,
            &format!("STOP:{:.2}", signal.predicted.stop_loss),
            x,
            stop_y,
            label_scale,
            color,
            color,
        );

        // stop_percent
        let mut stop_percent = stop_percent;
        if signal.predicted.direction == "short" {
            stop_percent *= -1.0;
        }
        let prefix = if stop_percent > 0.0 { "+" } else { "" };
        let _ = draw_label(
            img,
            font,
            &format!("{}{:.2}%", prefix, stop_percent),
            x + 1.0,
            stop_percent_y,
            label_scale,
            color,
            Some(Rgb([BLACK.0, BLACK.1, BLACK.2])),
        );

        // entry
        let _ = draw_label(
            img,
            font,
            &format!(
                "{}:{:.2}",
                signal.predicted.direction.to_uppercase(),
                signal.predicted.entry_price
            ),
            x,
            entry_y,
            label_scale,
            Rgb([BLACK.0, BLACK.1, BLACK.2]),
            Some(color),
        );

        // target_percent
        let mut target_percent = target_percent;
        if signal.predicted.direction == "short" {
            target_percent *= -1.0;
        }
        let prefix = if target_percent > 0.0 { "+" } else { "" };
        let _ = draw_label(
            img,
            font,
            &format!("{}{:.2}%", prefix, target_percent),
            x + 1.0,
            target_percent_y,
            label_scale,
            color,
            Some(Rgb([BLACK.0, BLACK.1, BLACK.2])),
        );

        // target
        let _ = draw_hallow_label(
            img,
            font,
            &format!("TAKE:{:.2}", signal.predicted.target_price),
            x,
            target_y,
            label_scale,
            color,
            color,
        );
    });

    Ok(())
}
