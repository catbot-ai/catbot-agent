use crate::charts::helpers::{get_visible_range_and_data, parse_timeframe_duration};
use ab_glyph::ScaleFont;
use ab_glyph::{Font, PxScale};
use chrono::DateTime;
use chrono_tz::Tz;
use common::numbers::{convert_grouped_data, group_by_fractional_part_f32, FractionalPart};
use common::{Kline, OrderBook};
use image::{ImageBuffer, Rgb};
use imageproc::drawing::{draw_filled_rect_mut, draw_line_segment_mut, draw_text_mut, text_size};
use imageproc::rect::Rect;
use m4rs::{bolinger_band, macd, Candlestick as M4rsCandlestick};
use plotters::coord::types::RangedCoordf32;
use plotters::prelude::*;
use plotters::style::full_palette::{GREEN_100, RED_100};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::error::Error;

use super::candle::*;
use super::helpers::{kline_to_m4rs_candlestick, parse_kline_time};

const B_RED: RGBColor = RGBColor(245, 71, 95);
const B_GREEN: RGBColor = RGBColor(17, 203, 129);
const B_BLACK: RGBColor = RGBColor(22, 26, 30);

// BB
const BB_BOUND: RGBColor = RGBColor(33, 88, 243);
const BB_MIDDLE: RGBColor = RGBColor(255, 185, 2);

// MCAD
const MCAD: RGBColor = RGBColor(34, 150, 243);
const MCAD_SIGNAL: RGBColor = RGBColor(255, 109, 1);

// SRSI
const SRSI_K: RGBColor = RGBColor(34, 150, 243);
const SRSI_D: RGBColor = RGBColor(255, 109, 1);

// Axis
const AXIS_SCALE: PxScale = PxScale { x: 20.0, y: 20.0 };

// Label
const HEAD_SCALE: PxScale = PxScale { x: 22.0, y: 22.0 };
const LABEL_COLOR: Rgb<u8> = Rgb([255, 255, 255]);
const LABEL_SCALE: PxScale = PxScale { x: 20.0, y: 20.0 };

// TODO: TRANSPARENT
const TRANSPARENT_BLACK_50: Rgb<u8> = Rgb([0, 0, 0]);
const PRICE_BG_COLOR: Rgb<u8> = Rgb([255, 255, 0]);
const PRICE_TEXT_COLOR: Rgb<u8> = Rgb([22, 26, 30]);

// Order
const BID_COLOR: RGBColor = B_GREEN;
const ASK_COLOR: RGBColor = B_RED;
const ORDER_LABEL_SCALE: PxScale = PxScale { x: 17.0, y: 17.0 };
const NUM_WHITE: Rgb<u8> = Rgb([255, 255, 255]);
const NUM_RED: Rgb<u8> = Rgb([B_RED.0, B_RED.1, B_RED.2]);
const NUM_GREEN: Rgb<u8> = Rgb([B_GREEN.0, B_GREEN.1, B_GREEN.2]);

const PRICE_LINE_COLOR: Rgb<u8> = PRICE_BG_COLOR;

#[allow(clippy::too_many_arguments, unused)]
pub fn draw_chart(
    root: &mut DrawingArea<BitMapBackend<'_>, plotters::coord::Shift>,
    all_candle_data: &[Kline],
    past_data: &[Kline],
    timezone: &Tz,
    chart: &Chart,
    min_price: f32,
    max_price: f32,
    first_time: DateTime<Tz>,
    last_time: DateTime<Tz>,
    margin_right: u32,
    candle_width: u32,
    final_width: u32,
) -> Result<(), Box<dyn Error>> {
    root.fill(&B_BLACK)?;

    let (top, bottom) = root.split_vertically((50).percent());

    let mut top_chart = ChartBuilder::on(&top)
        .margin_right(margin_right)
        .build_cartesian_2d(first_time..last_time, min_price * 0.95..max_price * 1.05)?;

    draw_candlesticks(
        &mut top_chart,
        all_candle_data,
        timezone,
        |is_bullish| {
            if is_bullish {
                B_GREEN.into()
            } else {
                B_RED.into()
            }
        },
        candle_width,
    )?;

    if chart.bollinger_enabled {
        draw_bollinger_bands(&mut top_chart, past_data, timezone)?;
    }

    if chart.volume_enabled || chart.macd_enabled || chart.stoch_rsi_enabled {
        let num_indicators = [
            chart.volume_enabled,
            chart.macd_enabled,
            chart.stoch_rsi_enabled,
        ]
        .iter()
        .filter(|&&enabled| enabled)
        .count() as f32;
        let section_height_percent = (100.0 / num_indicators).round() as u32;
        println!("section_height_percent: {:?}", section_height_percent);

        let mut remaining_area = bottom;
        let mut areas = Vec::new();

        if chart.volume_enabled {
            let (volume_area, rest) =
                remaining_area.split_vertically((section_height_percent).percent());
            areas.push(volume_area);
            remaining_area = rest;
        }

        if chart.macd_enabled {
            let (macd_area, rest) = remaining_area.split_vertically((50).percent());
            areas.push(macd_area);
            remaining_area = rest;
        }

        if chart.stoch_rsi_enabled {
            areas.push(remaining_area);
        }

        let mut area_iter = areas.into_iter().enumerate();

        if chart.volume_enabled {
            let (_idx, volume_area) = area_iter.next().unwrap();
            let (first_visible_time, last_visible_time, visible_data) =
                get_visible_range_and_data(past_data, timezone, candle_width, final_width)?;
            let max_volume = visible_data
                .iter()
                .map(|k| k.volume.parse::<f32>().unwrap())
                .fold(0.0f32, |a, b| a.max(b));
            let mut volume_chart = ChartBuilder::on(&volume_area)
                .margin_right(margin_right)
                .build_cartesian_2d(
                    first_visible_time..last_visible_time,
                    0.0f32..max_volume * 1.1,
                )?;
            draw_volume_bars(
                &mut volume_chart,
                &Some(visible_data.into_iter().collect()),
                timezone,
                &chart.timeframe,
            )?;
        }

        if chart.macd_enabled {
            let (_idx, macd_area) = area_iter.next().unwrap();
            let (first_visible_time, last_visible_time, visible_data) =
                get_visible_range_and_data(past_data, timezone, candle_width, final_width)?;
            let past_m4rs_candles: Vec<M4rsCandlestick> =
                visible_data.iter().map(kline_to_m4rs_candlestick).collect();
            let macd_result = macd(&past_m4rs_candles, 12, 26, 9)?;
            let macd_values: Vec<f32> = macd_result
                .iter()
                .flat_map(|entry| {
                    vec![
                        entry.macd as f32,
                        entry.signal as f32,
                        entry.histogram as f32,
                    ]
                })
                .collect();
            let macd_min = macd_values
                .iter()
                .fold(f32::INFINITY, |a, &b| a.min(b))
                .min(-1.0);
            let macd_max = macd_values
                .iter()
                .fold(f32::NEG_INFINITY, |a, &b| a.max(b))
                .max(1.0);
            let mut macd_chart = ChartBuilder::on(&macd_area)
                .margin_right(margin_right)
                .build_cartesian_2d(first_visible_time..last_visible_time, macd_min..macd_max)?;
            draw_macd(
                &mut macd_chart,
                &Some(visible_data.into_iter().collect()),
                timezone,
                &chart.timeframe,
            )?;
        }

        if chart.stoch_rsi_enabled {
            let (_idx, stoch_rsi_area) = area_iter.next().unwrap();
            let (first_visible_time, last_visible_time, visible_data) =
                get_visible_range_and_data(past_data, timezone, candle_width, final_width)?;
            let mut stoch_rsi_chart = ChartBuilder::on(&stoch_rsi_area)
                .margin_right(margin_right)
                .build_cartesian_2d(first_visible_time..last_visible_time, 0.0f32..100.0f32)?;
            let past_m4rs_candles: Vec<M4rsCandlestick> =
                visible_data.iter().map(kline_to_m4rs_candlestick).collect();
            let stoch_rsi_result = m4rs::stochastics(&past_m4rs_candles, 14, 3)?;
            let stoch_rsi_lines: Vec<(DateTime<Tz>, f32, f32)> = stoch_rsi_result
                .iter()
                .map(|entry| {
                    let t = parse_kline_time(entry.at as i64, timezone);
                    (t, entry.k as f32, entry.d as f32)
                })
                .collect();
            let k_style = ShapeStyle::from(&SRSI_K).stroke_width(1);
            let d_style = ShapeStyle::from(&SRSI_D).stroke_width(1);
            stoch_rsi_chart.draw_series(LineSeries::new(
                stoch_rsi_lines.iter().map(|(t, k, _)| (*t, *k)),
                k_style,
            ))?;
            stoch_rsi_chart.draw_series(LineSeries::new(
                stoch_rsi_lines.iter().map(|(t, _, d)| (*t, *d)),
                d_style,
            ))?;
            let upper_line = 80.0f32;
            let lower_line = 20.0f32;
            let dash_style = ShapeStyle {
                color: WHITE.mix(1.0),
                filled: false,
                stroke_width: 1,
            };
            stoch_rsi_chart
                .draw_series(DashedLineSeries::new(
                    vec![
                        (first_visible_time, upper_line),
                        (last_visible_time, upper_line),
                    ],
                    5,
                    10,
                    dash_style,
                ))
                .unwrap();
            stoch_rsi_chart
                .draw_series(DashedLineSeries::new(
                    vec![
                        (first_visible_time, lower_line),
                        (last_visible_time, lower_line),
                    ],
                    5,
                    10,
                    dash_style,
                ))
                .unwrap();
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments, unused)]
pub fn draw_axis_labels(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    font: &impl Font,
    past_data: &[Kline],
    chart: &Chart,
    height: u32,
    final_width: u32,
    margin_right: u32,
    min_price: f32,
    max_price: f32,
) -> Result<(i32), Box<dyn Error>> {
    let white = Rgb([255u8, 255u8, 255u8]);
    let label_scale = AXIS_SCALE;
    let font_metrics = font.as_scaled(label_scale);
    let text_x = (final_width - margin_right + 6) as i32;
    let text_height = (font_metrics.ascent() - font_metrics.descent()).ceil() as i32;

    let num_indicators = [
        chart.volume_enabled,
        chart.macd_enabled,
        chart.stoch_rsi_enabled,
    ]
    .iter()
    .filter(|&&enabled| enabled)
    .count() as f32;
    let section_height = height as f32 * 0.5 / num_indicators;
    let top_section_height = height as f32 * 0.5;

    // Add price labels for the candlestick section
    let price_range = max_price * 1.05 - min_price * 0.95;
    let price_step = price_range / 2.0;
    let price_y_positions = [
        0.0,
        top_section_height * 0.5,
        top_section_height - text_height as f32,
    ];
    for (i, y) in price_y_positions.iter().enumerate() {
        let price = max_price * 1.05 - (i as f32 * price_step);
        draw_label(
            img,
            font,
            &format!("{:.2}", price),
            text_x,
            *y as i32,
            label_scale,
            white,
            TRANSPARENT_BLACK_50,
        )?;
    }

    // Add current price label with refined y-position mapping
    let mut current_price_y = 0;
    if let Some(last_candle) = past_data.last() {
        let current_price = last_candle.close_price.parse::<f32>().unwrap();
        let adjusted_min_price = min_price * 0.95;
        let adjusted_max_price = max_price * 1.05;
        let price_range_adjusted = adjusted_max_price - adjusted_min_price;

        // Map current_price to y-position within the candlestick section
        let normalized_position = (current_price - adjusted_min_price) / price_range_adjusted;
        let y_position =
            2 + (top_section_height * (1.0 - normalized_position)) as i32 - text_height / 2;

        // Constrain y-position to stay within the candlestick section
        let y_position_clamped = y_position
            .max(0)
            .min((top_section_height - text_height as f32) as i32);

        current_price_y = y_position_clamped;

        draw_label(
            img,
            font,
            &format!("{:.2}", current_price),
            text_x,
            y_position_clamped,
            label_scale,
            PRICE_TEXT_COLOR,
            PRICE_BG_COLOR,
        )?;
    }

    let mut current_y = top_section_height;

    if chart.volume_enabled {
        let volumes: Vec<f32> = past_data
            .iter()
            .map(|k| k.volume.parse::<f32>().unwrap())
            .collect();
        let max_volume = volumes.iter().fold(0.0f32, |a, &b| a.max(b));
        let max_volume_display = max_volume * 1.1;
        let volume_step = max_volume_display / 2.0;
        let volume_y_positions = [
            current_y,
            current_y + section_height * 0.5,
            current_y + section_height - text_height as f32,
        ];
        for (i, y) in volume_y_positions.iter().enumerate() {
            let volume = max_volume_display - (i as f32 * volume_step);
            draw_label(
                img,
                font,
                &format!("{:.0}k", volume / 1000.0),
                text_x,
                *y as i32,
                label_scale,
                white,
                TRANSPARENT_BLACK_50,
            )?;
        }
        current_y += section_height;
    }

    if chart.macd_enabled {
        let past_m4rs_candles: Vec<M4rsCandlestick> =
            past_data.iter().map(kline_to_m4rs_candlestick).collect();
        let macd_result = macd(&past_m4rs_candles, 12, 26, 9)?;
        let macd_values: Vec<f32> = macd_result
            .iter()
            .flat_map(|entry| {
                vec![
                    entry.macd as f32,
                    entry.signal as f32,
                    entry.histogram as f32,
                ]
            })
            .collect();
        let macd_min = macd_values
            .iter()
            .fold(f32::INFINITY, |a, &b| a.min(b))
            .min(-1.0);
        let macd_max = macd_values
            .iter()
            .fold(f32::NEG_INFINITY, |a, &b| a.max(b))
            .max(1.0);
        let macd_step = (macd_max - macd_min) / 2.0;
        let macd_y_positions = [
            current_y,
            current_y + section_height * 0.5,
            current_y + section_height - text_height as f32,
        ];
        for (i, y) in macd_y_positions.iter().enumerate() {
            let macd_value = macd_max - (i as f32 * macd_step);
            draw_label(
                img,
                font,
                &format!("{:.2}", macd_value),
                text_x,
                *y as i32,
                label_scale,
                white,
                TRANSPARENT_BLACK_50,
            )?;
        }
        current_y += section_height;
    }

    if chart.stoch_rsi_enabled {
        let stoch_rsi_step = 100.0 / 2.0;
        let stoch_rsi_y_positions = [
            current_y,
            current_y + section_height * 0.5,
            current_y + section_height - text_height as f32,
        ];
        for (i, y) in stoch_rsi_y_positions.iter().enumerate() {
            let stoch_rsi_value = 100.0 - (i as f32 * stoch_rsi_step);
            draw_label(
                img,
                font,
                &format!("{:.0}", stoch_rsi_value),
                text_x,
                *y as i32,
                label_scale,
                white,
                TRANSPARENT_BLACK_50,
            )?;
        }
    }

    Ok(current_price_y)
}

pub fn draw_lines(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    chart: &Chart,
    width: u32,
    height: u32,
) -> Result<(), Box<dyn Error>> {
    let root = BitMapBackend::with_buffer(img, (width, height)).into_drawing_area();
    let root = root.apply_coord_spec(Cartesian2d::<RangedCoordf32, RangedCoordf32>::new(
        0f32..1f32,
        0f32..1f32,
        (0..width as i32, 0..height as i32),
    ));

    if !chart.lines.is_empty() {
        let style = chart.line_style.clone().unwrap_or(LineStyle {
            stroke_width: 2,
            color: YELLOW,
        });
        for &[p1, p2] in chart.lines.iter() {
            root.draw(&PathElement::new(
                vec![p1, p2],
                ShapeStyle::from(&style.color).stroke_width(style.stroke_width as u32),
            ))?;
        }
    }

    if !chart.points.is_empty() {
        let style = chart.point_style.clone().unwrap_or(PointStyle {
            radius: 3,
            color: WHITE,
        });
        for &(x, y) in chart.points.iter() {
            root.draw(&Circle::new(
                (x, y),
                style.radius,
                ShapeStyle::from(&style.color).filled(),
            ))?;
        }
    }

    Ok(())
}

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
            let x_pos = (*x * final_width as f32) as i32 + style.offset_x;
            let y_pos = (*y * height as f32) as i32 + style.offset_y;
            draw_label(
                img,
                font,
                text,
                x_pos,
                y_pos,
                style.scale,
                style.color,
                style.background_color,
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
    x: i32,
    y: i32,
    scale: PxScale,
    color: Rgb<u8>,
    background_color: Rgb<u8>,
) -> anyhow::Result<()> {
    let font_metrics = font.as_scaled(scale);
    let (text_width, text_height) = text_size(scale, font, text);
    let padding = 2f32 * scale.x / text_height as f32;

    draw_filled_rect_mut(
        img,
        Rect::at(x, y).of_size(
            text_width + 2 * padding as u32,
            text_height + 2 * padding as u32,
        ),
        background_color,
    );

    draw_text_mut(
        img,
        color,
        (x as f32 + padding) as i32,
        (y as f32 + padding + font_metrics.descent() / text_height as f32 * scale.y * 0.6) as i32,
        scale,
        font,
        text,
    );

    Ok(())
}

// Drawing helpers
pub fn draw_candlesticks<F>(
    chart: &mut ChartContext<
        '_,
        BitMapBackend<'_>,
        Cartesian2d<RangedDateTime<DateTime<Tz>>, RangedCoordf32>,
    >,
    candle_data: &[Kline],
    timezone: &Tz,
    color_selector: F,
    candle_width: u32,
) -> Result<(), Box<dyn Error>>
where
    F: Fn(bool) -> RGBAColor,
{
    chart.draw_series(candle_data.iter().map(|k| {
        let time = parse_kline_time(k.open_time, timezone);
        let open = k.open_price.parse::<f32>().unwrap();
        let high = k.high_price.parse::<f32>().unwrap();
        let low = k.low_price.parse::<f32>().unwrap();
        let close = k.close_price.parse::<f32>().unwrap();
        let is_bullish = close >= open;
        let color = color_selector(is_bullish);
        // Use candle_width for the width of each candlestick
        CandleStick::new(
            time,
            open,
            high,
            low,
            close,
            ShapeStyle::from(&color).filled(),
            ShapeStyle::from(&color).filled(),
            candle_width, // Ensure integer for CandleStick
        )
    }))?;
    Ok(())
}

pub fn draw_candle_detail(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    chart: &Chart,
    font: &impl Font,
) -> Result<(), Box<dyn Error>> {
    if let Some(past_candle_data) = &chart.past_candle_data {
        let latest_candle = past_candle_data.last().unwrap();
        let open = latest_candle.open_price.parse::<f32>().unwrap();
        let high = latest_candle.high_price.parse::<f32>().unwrap();
        let low = latest_candle.low_price.parse::<f32>().unwrap();
        let close = latest_candle.close_price.parse::<f32>().unwrap();
        let change = (close - open) / open * 100.0;
        let candle_detail = format!(
            "{} {} O {:.2} H {:.2} L {:.2} C {:.2} {} ({:.2}%)",
            chart.metadata.title.split(' ').next().unwrap_or(""),
            chart.timeframe,
            open,
            high,
            low,
            close,
            if change >= 0.0 { "+" } else { "" },
            change
        );
        draw_label(
            img,
            font,
            &candle_detail,
            10,
            10,
            HEAD_SCALE,
            LABEL_COLOR,
            TRANSPARENT_BLACK_50,
        )?;
    }
    Ok(())
}

pub fn draw_bollinger_bands(
    chart: &mut ChartContext<
        '_,
        BitMapBackend<'_>,
        Cartesian2d<RangedDateTime<DateTime<Tz>>, RangedCoordf32>,
    >,
    past_data: &[Kline],
    timezone: &Tz,
) -> Result<(), Box<dyn Error>> {
    if !past_data.is_empty() {
        let past_m4rs_candles: Vec<M4rsCandlestick> =
            past_data.iter().map(kline_to_m4rs_candlestick).collect();
        let past_bb_result = bolinger_band(&past_m4rs_candles, 20)?;
        let past_bb_lines: Vec<(DateTime<Tz>, f32, f32, f32)> = past_bb_result
            .iter()
            .map(|entry| {
                let t = parse_kline_time(entry.at as i64, timezone);
                let avg = entry.avg as f32;
                let upper = (entry.avg + 2.0 * entry.sigma) as f32;
                let lower = (entry.avg - 2.0 * entry.sigma) as f32;
                (t, avg, upper, lower)
            })
            .collect();

        let bound_style = ShapeStyle::from(&BB_BOUND).stroke_width(1);
        let avg_style = ShapeStyle::from(&BB_MIDDLE).stroke_width(1);
        chart.draw_series(LineSeries::new(
            past_bb_lines.iter().map(|(t, avg, _, _)| (*t, *avg)),
            avg_style,
        ))?;
        chart.draw_series(LineSeries::new(
            past_bb_lines.iter().map(|(t, _, upper, _)| (*t, *upper)),
            bound_style,
        ))?;
        chart.draw_series(LineSeries::new(
            past_bb_lines.iter().map(|(t, _, _, lower)| (*t, *lower)),
            bound_style,
        ))?;
    }
    Ok(())
}

pub fn draw_bollinger_detail(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    past_data: &[Kline],
    font: &impl Font,
) -> Result<(), Box<dyn Error>> {
    if !past_data.is_empty() {
        let past_m4rs_candles: Vec<M4rsCandlestick> =
            past_data.iter().map(kline_to_m4rs_candlestick).collect();
        let bb_result = bolinger_band(&past_m4rs_candles, 20)?;
        let latest_bb = bb_result.last().unwrap();
        let ma_7 = past_m4rs_candles
            .iter()
            .rev()
            .take(7)
            .map(|c| c.close)
            .sum::<f64>()
            / 7.0;
        let ma_25 = past_m4rs_candles
            .iter()
            .rev()
            .take(25)
            .map(|c| c.close)
            .sum::<f64>()
            / 25.0;
        let ma_99 = past_m4rs_candles
            .iter()
            .rev()
            .take(99)
            .map(|c| c.close)
            .sum::<f64>()
            / 99.0;
        let ta_detail = format!(
            "MA 7 close 0 SMA 9 {:.2}\nMA 25 close 0 SMA 9 {:.2}\nMA 99 close 0 SMA 9 {:.2}\nBB 20 2 {:.2} {:.2} {:.2}",
            ma_7, ma_25, ma_99, latest_bb.avg, latest_bb.avg + 2.0 * latest_bb.sigma, latest_bb.avg - 2.0 * latest_bb.sigma
        );
        let mut y_offset = 50;
        for line in ta_detail.lines() {
            draw_label(
                img,
                font,
                line,
                10,
                y_offset,
                LABEL_SCALE,
                LABEL_COLOR,
                TRANSPARENT_BLACK_50,
            )?;
            y_offset += 25;
        }
    }
    Ok(())
}

pub fn draw_volume_bars(
    chart: &mut ChartContext<
        '_,
        BitMapBackend<'_>,
        Cartesian2d<RangedDateTime<DateTime<Tz>>, RangedCoordf32>,
    >,
    past_candle_data: &Option<Vec<Kline>>,
    timezone: &Tz,
    timeframe: &str,
) -> Result<(), Box<dyn Error>> {
    if let Some(past_data) = past_candle_data {
        chart
            .configure_mesh()
            .light_line_style(BLACK)
            .x_max_light_lines(1)
            .y_max_light_lines(1)
            .draw()?;
        chart.draw_series(past_data.iter().flat_map(|k| {
            let time: DateTime<Tz> = parse_kline_time(k.open_time, timezone);
            let volume = k.volume.parse::<f32>().unwrap();
            let bar_width = parse_timeframe_duration(timeframe);
            let open = k.open_price.parse::<f32>().unwrap();
            let close = k.close_price.parse::<f32>().unwrap();
            let is_bullish = close >= open;
            let fill_color = if is_bullish { B_GREEN } else { B_RED };
            let fill_style = ShapeStyle {
                color: fill_color.into(),
                filled: true,
                stroke_width: 0,
            };
            let stroke_style = ShapeStyle {
                color: B_BLACK.into(),
                filled: false,
                stroke_width: 1,
            };

            let filled_rect = Rectangle::new([(time, 0.0), (time + bar_width, volume)], fill_style);
            let stroked_rect =
                Rectangle::new([(time, 0.0), (time + bar_width, volume)], stroke_style);

            vec![filled_rect, stroked_rect]
        }))?;
    }
    Ok(())
}

pub fn draw_volume_detail(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    past_data: &[Kline],
    font: &impl Font,
    current_y: i32,
) -> Result<(), Box<dyn Error>> {
    if !past_data.is_empty() {
        let volume_sma: f32 = past_data
            .iter()
            .rev()
            .take(9)
            .map(|k| k.volume.parse::<f32>().unwrap())
            .sum::<f32>()
            / 9.0;
        let volume_detail = format!("Volume SMA 9 {:.2}K", volume_sma / 1000.0);
        draw_label(
            img,
            font,
            &volume_detail,
            10,
            current_y,
            LABEL_SCALE,
            LABEL_COLOR,
            TRANSPARENT_BLACK_50,
        )?;
    }
    Ok(())
}

pub fn draw_macd(
    chart: &mut ChartContext<
        '_,
        BitMapBackend<'_>,
        Cartesian2d<RangedDateTime<DateTime<Tz>>, RangedCoordf32>,
    >,
    past_candle_data: &Option<Vec<Kline>>,
    timezone: &Tz,
    timeframe: &str,
) -> Result<(), Box<dyn Error>> {
    chart
        .configure_mesh()
        .light_line_style(BLACK)
        .x_max_light_lines(1)
        .y_max_light_lines(1)
        .draw()?;

    if let Some(past_data) = past_candle_data {
        let past_m4rs_candles: Vec<M4rsCandlestick> =
            past_data.iter().map(kline_to_m4rs_candlestick).collect();
        let macd_result = macd(&past_m4rs_candles, 12, 26, 9)?;
        let macd_lines: Vec<(DateTime<Tz>, f32, f32, f32)> = macd_result
            .iter()
            .map(|entry| {
                let t = parse_kline_time(entry.at as i64, timezone);
                (
                    t,
                    entry.macd as f32,
                    entry.signal as f32,
                    entry.histogram as f32,
                )
            })
            .collect();

        let m_style = ShapeStyle::from(&MCAD).stroke_width(1);
        let s_style = ShapeStyle::from(&MCAD_SIGNAL).stroke_width(1);
        chart.draw_series(LineSeries::new(
            macd_lines.iter().map(|(t, m, _, _)| (*t, *m)),
            m_style,
        ))?;
        chart.draw_series(LineSeries::new(
            macd_lines.iter().map(|(t, _, s, _)| (*t, *s)),
            s_style,
        ))?;

        let plotting_area = chart.plotting_area();
        let mut previous_h: Option<f32> = None;
        let bar_width = parse_timeframe_duration(timeframe);

        for (t, _, _, h) in macd_lines.iter() {
            let is_lower = previous_h.map_or(false, |prev| *h < prev);

            let fill_color = if *h > 0.0 {
                if is_lower {
                    B_GREEN
                } else {
                    GREEN_100
                }
            } else if is_lower {
                B_RED
            } else {
                RED_100
            };
            let fill_style = ShapeStyle {
                color: fill_color.into(),
                filled: true,
                stroke_width: 0,
            };
            let stroke_style = ShapeStyle {
                color: B_BLACK.into(),
                filled: false,
                stroke_width: 1,
            };

            plotting_area.draw(&Rectangle::new(
                [(*t, 0.0), (*t + bar_width, *h)],
                fill_style,
            ))?;
            plotting_area.draw(&Rectangle::new(
                [(*t, 0.0), (*t + bar_width, *h)],
                stroke_style,
            ))?;
            previous_h = Some(*h);
        }
    }
    Ok(())
}

pub fn draw_macd_detail(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    past_data: &[Kline],
    font: &impl Font,
    current_y: i32,
) -> Result<(), Box<dyn Error>> {
    if !past_data.is_empty() {
        let past_m4rs_candles: Vec<M4rsCandlestick> =
            past_data.iter().map(kline_to_m4rs_candlestick).collect();
        let macd_result = macd(&past_m4rs_candles, 12, 26, 9)?;
        let latest_macd = macd_result.last().unwrap();
        let macd_detail = format!(
            "MACD 12 26 close 9 {:.2} {:.2} {:.2}",
            latest_macd.macd, latest_macd.signal, latest_macd.histogram
        );
        draw_label(
            img,
            font,
            &macd_detail,
            10,
            current_y,
            LABEL_SCALE,
            LABEL_COLOR,
            TRANSPARENT_BLACK_50,
        )?;
    }
    Ok(())
}

pub fn draw_stoch_rsi_detail(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    past_data: &[Kline],
    font: &impl Font,
    current_y: i32,
) -> Result<(), Box<dyn Error>> {
    if !past_data.is_empty() {
        let past_m4rs_candles: Vec<M4rsCandlestick> =
            past_data.iter().map(kline_to_m4rs_candlestick).collect();
        let stoch_rsi_result = m4rs::slow_stochastics(&past_m4rs_candles, 14, 3, 5)?;
        let latest_stoch_rsi = stoch_rsi_result.last().unwrap();
        let stoch_rsi_detail = format!(
            "Stoch RSI 14 14 3 {:.2} {:.2}",
            latest_stoch_rsi.k, latest_stoch_rsi.d
        );
        draw_label(
            img,
            font,
            &stoch_rsi_detail,
            10,
            current_y,
            LABEL_SCALE,
            LABEL_COLOR,
            TRANSPARENT_BLACK_50,
        )?;
    }
    Ok(())
}

pub fn draw_point_on_candle(
    chart: &mut ChartContext<
        '_,
        BitMapBackend<'_>,
        Cartesian2d<RangedDateTime<DateTime<Tz>>, RangedCoordf32>,
    >,
    timezone: &Tz,
    long_signals: &[(i64, i64, f32, f32)], // (entry_time, target_time, entry_price, target_price)
    short_signals: &[(i64, i64, f32, f32)], // (entry_time, target_time, entry_price, target_price)
) -> Result<(), Box<dyn Error>> {
    // Draw long signals (green)
    let long_circle_style = ShapeStyle::from(&GREEN).filled();
    let long_line_style = ShapeStyle::from(&GREEN).stroke_width(2);
    for &(entry_time, target_time, entry_price, target_price) in long_signals {
        let entry_dt = parse_kline_time(entry_time, timezone);
        let target_dt = parse_kline_time(target_time, timezone);

        // Draw circle at the entry point
        chart.draw_series(std::iter::once(Circle::new(
            (entry_dt, entry_price),
            5, // Radius of 5 pixels
            long_circle_style,
        )))?;

        // Draw circle at the target point
        chart.draw_series(std::iter::once(Circle::new(
            (target_dt, target_price),
            5, // Radius of 5 pixels
            long_circle_style,
        )))?;

        // Draw line connecting entry and target points
        chart.draw_series(LineSeries::new(
            vec![(entry_dt, entry_price), (target_dt, target_price)],
            long_line_style,
        ))?;
    }

    // Draw short signals (red)
    let short_circle_style = ShapeStyle::from(&RED).filled();
    let short_line_style = ShapeStyle::from(&RED).stroke_width(2);
    for &(entry_time, target_time, entry_price, target_price) in short_signals {
        let entry_dt = parse_kline_time(entry_time, timezone);
        let target_dt = parse_kline_time(target_time, timezone);

        // Draw circle at the entry point
        chart.draw_series(std::iter::once(Circle::new(
            (entry_dt, entry_price),
            5, // Radius of 5 pixels
            short_circle_style,
        )))?;

        // Draw circle at the target point
        chart.draw_series(std::iter::once(Circle::new(
            (target_dt, target_price),
            5, // Radius of 5 pixels
            short_circle_style,
        )))?;

        // Draw line connecting entry and target points
        chart.draw_series(LineSeries::new(
            vec![(entry_dt, entry_price), (target_dt, target_price)],
            short_line_style,
        ))?;
    }

    Ok(())
}

pub fn draw_order_book(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    font: &impl Font,
    orderbook_data: &OrderBook,
    min_price: f32,
    max_price: f32,
    width: u32,
    height: u32,
    current_price_y: i32,
    parent_offset_x: u32,
) -> Result<(), Box<dyn Error>> {
    let parent_offset_x = parent_offset_x + 80;
    let price_rect_height = 20;
    let price_rect_height_half = price_rect_height / 2;

    // Group the order book data f32 type.
    let (grouped_bids, grouped_asks): (HashMap<u32, f64>, HashMap<u32, f64>) =
        group_by_fractional_part_f32(orderbook_data, FractionalPart::One);

    // Transform group to hashmap
    let (bid_volumes, ask_volumes) =
        convert_grouped_data(&grouped_bids, &grouped_asks, min_price, max_price);

    // Prepare bid data for the histogram
    let mut bid_data: Vec<(f32, f32)> = bid_volumes
        .iter()
        .map(|(price_bits, volume)| (f32::from_bits(*price_bits), *volume))
        .collect();

    // Prepare ask data for the histogram
    let mut ask_data: Vec<(f32, f32)> = ask_volumes
        .iter()
        .map(|(price_bits, volume)| (f32::from_bits(*price_bits), *volume))
        .collect();

    // Sort ask_data by first element (price) in descending order
    ask_data.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));

    // Sort bid_data by first element (price) in descending order
    bid_data.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));

    // Prepare position for the histogram
    let mut current_y = -{ price_rect_height_half } / 2i32;
    let rect_height = 8u32;
    let gap = 8i32;
    let bar_height = rect_height as i32 + gap;
    // let offset_y = bar_height - gap / 2 + current_price_y
    //     - bar_height * (bid_data.len() as i32 + ask_data.len() as i32) / 2;
    let offset_y =
        current_price_y - bar_height * (ask_data.len() as i32) + bar_height - bar_height / 4 - 1;

    let max_bar_width = 64;
    let current_x = 32u32;

    let max_bid_volume = bid_data
        .iter()
        .map(|(_, volume)| *volume)
        .fold(0.0, f32::max);

    let max_ask_volume = ask_data
        .iter()
        .map(|(_, volume)| *volume)
        .fold(0.0, f32::max);
    let max_volume_width = max_bid_volume.max(max_ask_volume) as i32;

    let max_rect_width = (max_volume_width as f32 / max_bar_width as f32) as i32;
    let offset_x = parent_offset_x + current_x + 72u32;

    {
        let root = BitMapBackend::with_buffer(img, (width, height)).into_drawing_area();

        // Draw the ask histograms
        for (price, volume) in ask_data.iter() {
            if price.is_finite() && volume.is_finite() {
                let rect_width = (*volume / max_rect_width as f32) as i32;

                root.draw(&Rectangle::new(
                    [
                        (offset_x as i32, offset_y + current_y),
                        (
                            offset_x as i32 + rect_width,
                            offset_y + (current_y) + rect_height as i32,
                        ),
                    ],
                    ShapeStyle::from(&ASK_COLOR).filled(),
                ))?;

                current_y += (rect_height + gap as u32) as i32;
            }
        }

        current_y += bar_height / 2;

        // Draw the bid histograms
        for (price, volume) in bid_data.iter() {
            if price.is_finite() && volume.is_finite() {
                let rect_width = (*volume / max_rect_width as f32) as i32;

                root.draw(&Rectangle::new(
                    [
                        (offset_x as i32, offset_y + current_y),
                        (
                            offset_x as i32 + rect_width,
                            offset_y + current_y + rect_height as i32,
                        ),
                    ],
                    ShapeStyle::from(&BID_COLOR).filled(),
                ))?;

                current_y += (rect_height + gap as u32) as i32;
            }
        }
    }

    // Reset
    let mut current_y = -price_rect_height_half / 2i32;
    let offset_x = parent_offset_x;

    // Draw label
    for (price, volume) in ask_data.iter() {
        if price.is_finite() && volume.is_finite() {
            draw_label(
                img,
                font,
                &format!("{:.0}", price),
                offset_x as i32,
                offset_y + current_y,
                ORDER_LABEL_SCALE,
                NUM_RED,
                TRANSPARENT_BLACK_50,
            )?;

            draw_label(
                img,
                font,
                &format!("{:.2}", volume),
                (current_x + offset_x) as i32,
                offset_y + current_y,
                ORDER_LABEL_SCALE,
                NUM_WHITE,
                TRANSPARENT_BLACK_50,
            )?;

            current_y += rect_height as i32 + gap;
        }
    }
    current_y += price_rect_height_half;

    for (price, volume) in bid_data.iter() {
        if price.is_finite() && volume.is_finite() {
            draw_label(
                img,
                font,
                &format!("{:.0}", price),
                offset_x as i32,
                offset_y + current_y,
                ORDER_LABEL_SCALE,
                NUM_GREEN,
                TRANSPARENT_BLACK_50,
            )?;

            draw_label(
                img,
                font,
                &format!("{:.2}", volume),
                (current_x + offset_x) as i32,
                offset_y + current_y,
                ORDER_LABEL_SCALE,
                NUM_WHITE,
                TRANSPARENT_BLACK_50,
            )?;

            current_y += (rect_height + gap as u32) as i32;
        }
    }

    // Draw price line
    let price_line_y = current_price_y as f32 + price_rect_height_half as f32;
    draw_line_segment_mut(
        img,
        (parent_offset_x as f32 - 16f32, price_line_y),
        (width as f32, price_line_y),
        PRICE_LINE_COLOR,
    );

    Ok(())
}
