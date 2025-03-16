use image::{ImageBuffer, Rgb};
use imageproc::drawing::draw_line_segment_mut;

// Draw a dashed line segment on the image buffer
pub fn draw_dashed_line_segment_mut(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    start: (f32, f32),
    end: (f32, f32),
    dash_length: f32,
    gap_length: f32,
    color: Rgb<u8>,
) {
    let (x1, y1) = start;
    let (x2, y2) = end;

    // Calculate the total length of the line
    let dx = x2 - x1;
    let dy = y2 - y1;
    let length = (dx * dx + dy * dy).sqrt();

    // If the line is too short, just draw a solid line
    if length < dash_length + gap_length {
        draw_line_segment_mut(img, (x1, y1), (x2, y2), color);
        return;
    }

    // Calculate the number of dash-gap pairs
    let segment_length = dash_length + gap_length;
    let num_segments = 2 * (length / segment_length).round() as i32;

    // Calculate the direction vector
    let step_x = dx / length;
    let step_y = dy / length;

    let mut current_pos = 0.0;
    let mut is_dashing = true;

    for i in 0..num_segments {
        let start_pos = current_pos;
        let end_pos = if is_dashing {
            (start_pos + dash_length).min(length)
        } else {
            (start_pos + gap_length).min(length)
        };

        if is_dashing {
            let dash_start_x = x1 + step_x * start_pos;
            let dash_start_y = y1 + step_y * start_pos;
            let dash_end_x = x1 + step_x * end_pos;
            let dash_end_y = y1 + step_y * end_pos;

            draw_line_segment_mut(
                img,
                (dash_start_x, dash_start_y),
                (dash_end_x, dash_end_y),
                color,
            );
        }

        current_pos = end_pos;
        is_dashing = !is_dashing;
    }

    // Draw the remaining portion if there's any
    if current_pos < length && is_dashing {
        let dash_start_x = x1 + step_x * current_pos;
        let dash_start_y = y1 + step_y * current_pos;
        draw_line_segment_mut(img, (dash_start_x, dash_start_y), (x2, y2), color);
    }
}
