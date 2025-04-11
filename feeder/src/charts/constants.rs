pub use ab_glyph::PxScale;
pub use image::Rgb;
pub use plotters::prelude::RGBColor;

pub const B_RED: RGBColor = RGBColor(245, 71, 95);
pub const B_GREEN: RGBColor = RGBColor(17, 203, 129);
pub const B_GREEN_DIM: RGBColor = RGBColor(17 / 2, 203 / 2, 129 / 2);
pub const B_RED_DIM: RGBColor = RGBColor(245 / 2, 71 / 2, 95 / 2);
pub const B_BLACK: RGBColor = RGBColor(22, 26, 30);
// BB
pub const BB_UPPER_BOUND: RGBColor = RGBColor(34, 150, 243);
pub const BB_LOWER_BOUND: RGBColor = RGBColor(255, 109, 1);
pub const BB_MIDDLE: RGBColor = RGBColor(255, 185, 2);
pub const BB_UPPER_BOUND_LABEL: Rgb<u8> = Rgb([34, 150, 243]);
pub const BB_LOWER_BOUND_LABEL: Rgb<u8> = Rgb([255, 109, 1]);
// MCAD
pub const MCAD: RGBColor = RGBColor(34, 150, 243);
pub const MCAD_SIGNAL: RGBColor = RGBColor(255, 109, 1);
// SRSI
pub const SRSI_K: RGBColor = RGBColor(34, 150, 243);
pub const SRSI_D: RGBColor = RGBColor(255, 109, 1);
// Axis
pub const AXIS_SCALE: PxScale = PxScale { x: 20.0, y: 20.0 };
// Label
pub const HEAD_SCALE: PxScale = PxScale { x: 22.0, y: 22.0 };
pub const LABEL_COLOR: Rgb<u8> = Rgb([255, 255, 255]);
pub const LABEL_SCALE: PxScale = PxScale { x: 20.0, y: 20.0 };
// TODO: TRANSPARENT
pub const TRANSPARENT_BLACK_50: Rgb<u8> = Rgb([0, 0, 0]); // Note: This isn't transparent
pub const PRICE_BG_COLOR: Rgb<u8> = Rgb([255, 255, 0]);
pub const PRICE_TEXT_COLOR: Rgb<u8> = Rgb([22, 26, 30]);
// Order
pub const BID_COLOR: RGBColor = B_GREEN_DIM;
pub const ASK_COLOR: RGBColor = B_RED_DIM;
pub const ORDER_LABEL_SCALE: PxScale = PxScale { x: 18.0, y: 18.0 };
pub const NUM_WHITE: Rgb<u8> = Rgb([255, 255, 255]);
pub const NUM_RED: Rgb<u8> = Rgb([B_RED.0, B_RED.1, B_RED.2]);
pub const NUM_GREEN: Rgb<u8> = Rgb([B_GREEN.0, B_GREEN.1, B_GREEN.2]);
// Price Line
pub const PRICE_LINE_COLOR: Rgb<u8> = PRICE_BG_COLOR;
