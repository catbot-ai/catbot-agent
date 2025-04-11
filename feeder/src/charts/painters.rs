use super::candle::{calculate_candle_width, draw_candlesticks, Chart, LineStyle, PointStyle};
use super::helpers::parse_kline_time;

use super::indicators::{draw_bollinger_bands, draw_macd, draw_volume_bars};
use super::labels::draw_label;
use crate::charts::helpers::get_visible_range_and_data;
use ab_glyph::Font;
use ab_glyph::ScaleFont;
use chrono::DateTime;
use chrono_tz::Tz;
use common::m4rs::kline_to_m4rs_candlestick;

use common::rsi::calculate_stoch_rsi;
use common::Kline;
use image::{ImageBuffer, Rgb};

use imageproc::rect::Rect;
use m4rs::{macd, Candlestick as M4rsCandlestick};
use plotters::coord::types::RangedCoordf32;
use plotters::prelude::*;

use super::constants::*;
pub use plotters::style::full_palette::{WHITE, YELLOW};
use std::error::Error;

#[allow(clippy::too_many_arguments, unused)]
pub fn draw_chart(
    root: &mut DrawingArea<BitMapBackend<'_>, plotters::coord::Shift>,
    all_candle_data: &[Kline],
    klines: &[Kline],
    timezone: &Tz,
    chart: &Chart,
    min_price: f32,
    max_price: f32,
    first_time: DateTime<Tz>,
    last_time: DateTime<Tz>,
    margin_right: u32,
    final_width: u32,
    last_past_time: i64,
    timeframe: &str,
) -> Result<(f32, f32), Box<dyn Error>> {
    root.fill(&B_BLACK)?;

    let (top, bottom) = root.split_vertically((50).percent());

    let mut top_chart = ChartBuilder::on(&top)
        .margin_right(margin_right)
        .build_cartesian_2d(first_time..last_time, min_price * 0.95..max_price * 1.05)?;

    let total_candles_num = all_candle_data.len() as u8;
    let candle_width = calculate_candle_width(&top_chart, total_candles_num);

    draw_candlesticks(
        &mut top_chart,
        all_candle_data,
        timezone,
        |is_bullish, is_predicted| {
            let fill_color: RGBColor = if is_bullish {
                if is_predicted {
                    B_GREEN_DIM
                } else {
                    B_GREEN
                }
            } else if is_predicted {
                B_RED_DIM
            } else {
                B_RED
            };

            fill_color.into()
        },
        last_past_time,
        candle_width,
    )?;

    let (mut lower_bound, mut upper_bound) = (0.0, 0.0);
    if chart.bollinger_enabled {
        let (new_lower_bound, new_upper_bound) =
            draw_bollinger_bands(&mut top_chart, all_candle_data, timezone)?;
        lower_bound = new_lower_bound;
        upper_bound = new_upper_bound;
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
                get_visible_range_and_data(all_candle_data, timezone, candle_width, final_width)?;
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
                last_past_time,
            )?;
        }

        if chart.macd_enabled {
            let (_idx, macd_area) = area_iter.next().unwrap();
            let (first_visible_time, last_visible_time, visible_data) =
                get_visible_range_and_data(all_candle_data, timezone, candle_width, final_width)?;
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
                last_past_time,
            )?;
        }

        if chart.stoch_rsi_enabled {
            let (_idx, stoch_rsi_area) = area_iter.next().unwrap();
            let (first_visible_time, last_visible_time, visible_data) = get_visible_range_and_data(
                all_candle_data,
                timezone,
                candle_width,
                final_width * 2,
            )?;

            let mut stoch_rsi_chart = ChartBuilder::on(&stoch_rsi_area)
                .margin_right(margin_right)
                .build_cartesian_2d(first_visible_time..last_visible_time, 0.0f32..100.0f32)?;

            // Convert visible_data to M4rsCandlestick and calculate Stoch RSI
            let past_m4rs_candles: Vec<M4rsCandlestick> =
                visible_data.iter().map(kline_to_m4rs_candlestick).collect();
            let (_, stoch_rsi_k, stoch_rsi_d) =
                calculate_stoch_rsi(&past_m4rs_candles, 14, 14, 3, 3)?;

            // Align Stoch RSI values with timestamps
            let stoch_rsi_lines: Vec<(DateTime<Tz>, f32, f32)> = visible_data
                .iter()
                .enumerate()
                .filter_map(|(i, kline)| {
                    if i < stoch_rsi_k.len() && i < stoch_rsi_d.len() {
                        let t = parse_kline_time(kline.open_time, timezone);
                        Some((t, stoch_rsi_k[i] as f32, stoch_rsi_d[i] as f32))
                    } else {
                        None
                    }
                })
                .collect();

            // Draw %K line
            let k_style = ShapeStyle::from(&SRSI_K).stroke_width(1);
            stoch_rsi_chart.draw_series(LineSeries::new(
                stoch_rsi_lines.iter().map(|(t, k, _)| (*t, *k)),
                k_style,
            ))?;

            // Draw %D line
            let d_style = ShapeStyle::from(&SRSI_D).stroke_width(1);
            stoch_rsi_chart.draw_series(LineSeries::new(
                stoch_rsi_lines.iter().map(|(t, _, d)| (*t, *d)),
                d_style,
            ))?;

            // Draw upper and lower dashed lines
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

    Ok((lower_bound, upper_bound))
}

#[allow(clippy::too_many_arguments, unused)]
pub fn draw_axis_labels(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    font: &impl Font,
    klines: &[Kline],
    chart: &Chart,
    height: u32,
    final_width: u32,
    margin_right: u32,
    min_price: f32,
    max_price: f32,
) -> Result<Option<Rect>, Box<dyn Error>> {
    let white = Rgb([255u8, 255u8, 255u8]);
    let label_scale = AXIS_SCALE;
    let font_metrics = font.as_scaled(label_scale);
    let text_x = (final_width - margin_right + 6) as f32;
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
    // TODO: maybe too much here
    // for (i, y) in price_y_positions.iter().enumerate() {
    //     let price = max_price * 1.05 - (i as f32 * price_step);
    //     draw_label(
    //         img,
    //         font,
    //         &format!("{:.2}", price),
    //         text_x,
    //         *y,
    //         label_scale,
    //         white,
    //         TRANSPARENT_BLACK_50,
    //     )?;
    // }

    // Add current price label with refined y-position mapping
    let mut current_price_y = 0.0;
    let maybe_bounding_rect = if let Some(last_candle) = klines.last() {
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

        current_price_y = y_position_clamped as f32;

        // Draw current_price
        let price_bounding_rect = draw_label(
            img,
            font,
            &format!("{:.2}", current_price),
            text_x,
            y_position_clamped as f32,
            label_scale,
            PRICE_TEXT_COLOR,
            Some(PRICE_BG_COLOR),
        )?;

        Some(price_bounding_rect)
    } else {
        None
    };

    let mut current_y = top_section_height;

    if chart.volume_enabled {
        let volumes: Vec<f32> = klines
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
                *y,
                label_scale,
                white,
                Some(TRANSPARENT_BLACK_50),
            )?;
        }
        current_y += section_height;
    }

    if chart.macd_enabled {
        let past_m4rs_candles: Vec<M4rsCandlestick> =
            klines.iter().map(kline_to_m4rs_candlestick).collect();
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
                *y,
                label_scale,
                white,
                Some(TRANSPARENT_BLACK_50),
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
                *y,
                label_scale,
                white,
                Some(TRANSPARENT_BLACK_50),
            )?;
        }
    }

    Ok(maybe_bounding_rect)
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
