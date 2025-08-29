use super::helpers::{format_short_number, parse_kline_time};
use super::labels::draw_label;
use crate::charts::helpers::parse_interval_duration;
use ab_glyph::Font;
use chrono::DateTime;
use chrono_tz::Tz;
use common::m4rs::kline_to_m4rs_candlestick;
use common::numbers::{group_by_fractional_part, FractionalPart};
use common::rsi::{calculate_stoch_rsi, get_latest_bb_ma};
use common::{Kline, LongShortSignal, OrderBook};
use image::{ImageBuffer, Rgb};
use imageproc::drawing::draw_line_segment_mut;
use imageproc::rect::Rect;
use m4rs::{bolinger_band, macd, Candlestick as M4rsCandlestick};
use plotters::coord::types::RangedCoordf32;
use plotters::prelude::*;
pub use plotters::style::full_palette::{BLACK, GREEN_200, GREEN_900, RED_200, RED_900};

use std::cmp::Ordering;
use std::collections::HashMap;
use std::error::Error;

use super::constants::*;

pub fn draw_bollinger_bands(
    chart: &mut ChartContext<
        '_,
        BitMapBackend<'_>,
        Cartesian2d<RangedDateTime<DateTime<Tz>>, RangedCoordf32>,
    >,
    klines: &[Kline],
    timezone: &Tz,
) -> Result<(f32, f32), Box<dyn Error>> {
    if klines.is_empty() {
        // Handle empty case: return an error or default bounds
        return Err("No kline data provided to calculate Bollinger Bands".into());
    }

    let past_m4rs_candles: Vec<M4rsCandlestick> =
        klines.iter().map(kline_to_m4rs_candlestick).collect();
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

    let upper_bound_style = ShapeStyle::from(&BB_UPPER_BOUND).stroke_width(1);
    let lower_bound_style = ShapeStyle::from(&BB_LOWER_BOUND).stroke_width(1);
    let avg_style = ShapeStyle::from(&BB_MIDDLE).stroke_width(1);
    chart.draw_series(LineSeries::new(
        past_bb_lines.iter().map(|(t, avg, _, _)| (*t, *avg)),
        avg_style,
    ))?;
    chart.draw_series(LineSeries::new(
        past_bb_lines.iter().map(|(t, _, upper, _)| (*t, *upper)),
        upper_bound_style,
    ))?;
    chart.draw_series(LineSeries::new(
        past_bb_lines.iter().map(|(t, _, _, lower)| (*t, *lower)),
        lower_bound_style,
    ))?;

    // Get the last upper and lower bounds (default to 0.0 if no data, though this won't happen due to early return)
    let (_, _, upper_bound, lower_bound) = past_bb_lines.last().copied().unwrap_or((
        DateTime::<Tz>::MIN_UTC.with_timezone(timezone),
        0.0,
        0.0,
        0.0,
    ));

    Ok((lower_bound, upper_bound))
}

pub fn draw_bollinger_detail(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    klines: &[Kline],
    font: &impl Font,
) -> Result<(), Box<dyn Error>> {
    if !klines.is_empty() {
        let ma_bb_detail = get_latest_bb_ma(klines)?;
        let mut y_offset = 50.0;
        for line in ma_bb_detail.lines() {
            draw_label(
                img,
                font,
                line,
                10.0,
                y_offset,
                LABEL_SCALE,
                LABEL_COLOR,
                None,
            )?;
            y_offset += 25.0;
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
    maybe_klines: &Option<Vec<Kline>>,
    timezone: &Tz,
    interval: &str,
    last_past_time: i64,
) -> Result<(), Box<dyn Error>> {
    if let Some(klines) = maybe_klines {
        chart
            .configure_mesh()
            .light_line_style(BLACK)
            .x_max_light_lines(1)
            .y_max_light_lines(1)
            .draw()?;
        chart.draw_series(klines.iter().flat_map(|k| {
            let time: DateTime<Tz> = parse_kline_time(k.open_time, timezone);
            let volume = k.volume.parse::<f32>().unwrap();
            let bar_width = parse_interval_duration(interval);
            let open = k.open_price.parse::<f32>().unwrap();
            let close = k.close_price.parse::<f32>().unwrap();
            let is_bullish = close >= open;
            let is_predicted = last_past_time < k.open_time;
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
    klines: &[Kline],
    font: &impl Font,
    current_y: f32,
) -> Result<(), Box<dyn Error>> {
    if !klines.is_empty() {
        let volume_sma: f32 = klines
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
            10.0,
            current_y,
            LABEL_SCALE,
            LABEL_COLOR,
            Some(TRANSPARENT_BLACK_50),
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
    maybe_klines: &Option<Vec<Kline>>,
    timezone: &Tz,
    interval: &str,
    last_past_time: i64,
) -> Result<(), Box<dyn Error>> {
    chart
        .configure_mesh()
        .light_line_style(BLACK)
        .x_max_light_lines(1)
        .y_max_light_lines(1)
        .draw()?;

    if let Some(klines) = maybe_klines {
        let past_m4rs_candles: Vec<M4rsCandlestick> =
            klines.iter().map(kline_to_m4rs_candlestick).collect();
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
        let bar_width = parse_interval_duration(interval);

        for (t, _, _, h) in macd_lines.iter() {
            let is_lower = previous_h.map_or_else(|| false, |prev| *h < prev);
            let is_predicted = last_past_time < t.timestamp_millis();
            let fill_color = if is_predicted {
                if *h > 0.0 {
                    if is_lower {
                        B_GREEN_DIM
                    } else {
                        GREEN_900
                    }
                } else if is_lower {
                    B_RED_DIM
                } else {
                    RED_900
                }
            } else if *h > 0.0 {
                if is_lower {
                    B_GREEN
                } else {
                    GREEN_200
                }
            } else if is_lower {
                B_RED
            } else {
                RED_200
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
    klines: &[Kline],
    font: &impl Font,
    current_y: f32,
) -> Result<(), Box<dyn Error>> {
    if !klines.is_empty() {
        let past_m4rs_candles: Vec<M4rsCandlestick> =
            klines.iter().map(kline_to_m4rs_candlestick).collect();
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
            10.0,
            current_y,
            LABEL_SCALE,
            LABEL_COLOR,
            Some(TRANSPARENT_BLACK_50),
        )?;
    }
    Ok(())
}

pub fn draw_stoch_rsi_detail(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    klines: &[Kline],
    font: &impl Font,
    current_y: f32,
) -> Result<(), Box<dyn Error>> {
    if !klines.is_empty() {
        let past_m4rs_candles: Vec<M4rsCandlestick> =
            klines.iter().map(kline_to_m4rs_candlestick).collect();
        let (_, stoch_rsi_k, stoch_rsi_d) = calculate_stoch_rsi(&past_m4rs_candles, 14, 14, 3, 3)?;
        let stoch_rsi_detail = format!(
            "Stoch RSI 14 14 3 3 {:.2} {:.2}",
            stoch_rsi_k.last().unwrap(),
            stoch_rsi_d.last().unwrap()
        );
        draw_label(
            img,
            font,
            &stoch_rsi_detail,
            10.0,
            current_y,
            LABEL_SCALE,
            LABEL_COLOR,
            Some(TRANSPARENT_BLACK_50),
        )?;
    }
    Ok(())
}

pub fn draw_past_signals(
    chart: &mut ChartContext<
        '_,
        BitMapBackend<'_>,
        Cartesian2d<RangedDateTime<DateTime<Tz>>, RangedCoordf32>,
    >,
    timezone: &Tz,
    signals: &Vec<LongShortSignal>,
) -> Result<(), Box<dyn Error>> {
    // Draw long signals (green)
    let long_circle_style = ShapeStyle::from(&B_GREEN).filled();
    let long_line_style = ShapeStyle::from(&B_GREEN).stroke_width(2);

    let short_circle_style = ShapeStyle::from(&B_GREEN).filled();
    let short_line_style = ShapeStyle::from(&B_GREEN).stroke_width(2);

    for signal in signals {
        let entry_dt = parse_kline_time(signal.predicted.entry_time, timezone);
        let target_dt = parse_kline_time(signal.predicted.target_time, timezone);

        // Draw circle at the entry point
        chart.draw_series(std::iter::once(Circle::new(
            (entry_dt, signal.predicted.entry_price as f32),
            5, // Radius of 5 pixels
            long_circle_style,
        )))?;

        // Draw circle at the target point
        chart.draw_series(std::iter::once(Circle::new(
            (target_dt, signal.predicted.target_price as f32),
            5, // Radius of 5 pixels
            if signal.predicted.direction == "long" {
                long_circle_style
            } else {
                short_circle_style
            },
        )))?;

        // Draw line connecting entry and target points
        chart.draw_series(LineSeries::new(
            vec![
                (entry_dt, signal.predicted.entry_price as f32),
                (target_dt, signal.predicted.target_price as f32),
            ],
            if signal.predicted.direction == "long" {
                long_line_style
            } else {
                short_line_style
            },
        ))?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments, unused)]
pub fn draw_orderbook(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    font: &impl Font,
    orderbook: &OrderBook,
    min_price: f32,
    max_price: f32,
    width: u32,
    height: u32,
    parent_offset_x: f32,
    lower_bound: f32,
    upper_bound: f32,
    current_price_bounding_rect: Rect,
) -> Result<(HashMap<String, f32>), Box<dyn Error>> {
    // Output items y
    let mut bids_asks_y_map = HashMap::new();

    // Position
    let current_price_y = current_price_bounding_rect.top() as f32;
    let padding_right = 120.0;
    let parent_offset_x = parent_offset_x + padding_right;
    let price_rect_height = 20;
    let price_rect_height_half = price_rect_height / 2;

    // Group the order book data f32 type.
    let (grouped_bids, grouped_asks) = group_by_fractional_part(orderbook, FractionalPart::Two);

    // Prepare bid data for the histogram
    let mut bid_data: Vec<(f32, f32)> = grouped_bids
        .iter()
        .map(|(price_bits, volume)| (price_bits.parse::<f32>().unwrap(), *volume as f32))
        .collect();

    // Prepare ask data for the histogram
    let mut ask_data: Vec<(f32, f32)> = grouped_asks
        .iter()
        .map(|(price_bits, volume)| (price_bits.parse::<f32>().unwrap(), *volume as f32))
        .collect();

    // Sort ask_data by first element (price) in descending order
    ask_data.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));

    // Sort bid_data by first element (price) in descending order
    bid_data.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));

    // Prepare position for the histogram
    let mut current_y = (-{ price_rect_height_half } / 2i32);
    let histogram_rect_height = 17u32;
    let gap = 4i32;
    let bar_height = histogram_rect_height as i32 + gap;
    let offset_y =
        height as f32 / 2.0 - (bar_height * (ask_data.len() as i32) + bar_height - gap / 2) as f32;

    let max_bar_width = 80;
    let current_x = 40u32;

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
    let offset_x = parent_offset_x as u32 + current_x;

    {
        let width = img.width();
        let height = img.height();
        let mut img_rgb = img.clone().into_raw();

        {
            // Scope for root
            let root =
                BitMapBackend::with_buffer(&mut img_rgb, (width, height)).into_drawing_area();
            // Draw the ask histograms
            for (price, volume) in ask_data.iter() {
                if price.is_finite() && volume.is_finite() {
                    let rect_width = (*volume / max_rect_width as f32) as i32;
                    let color = if price.round() == upper_bound.round() {
                        BB_UPPER_BOUND
                    } else {
                        ASK_COLOR
                    };
                    let y = offset_y as i32 + current_y + histogram_rect_height as i32;
                    root.draw(&Rectangle::new(
                        [
                            (offset_x as i32, offset_y as i32 + current_y),
                            (offset_x as i32 + rect_width, y),
                        ],
                        ShapeStyle::from(color).filled(),
                    ))?;
                    current_y += (histogram_rect_height + gap as u32) as i32;
                    bids_asks_y_map.insert(price.to_string(), y as f32);
                }
            }
            current_y += bar_height / 2 - gap / 2 + 2;
            // Draw the bid histograms
            for (price, volume) in bid_data.iter() {
                if price.is_finite() && volume.is_finite() {
                    let rect_width = (*volume / max_rect_width as f32) as i32;
                    let color = if price.round() == lower_bound.round() {
                        BB_LOWER_BOUND
                    } else {
                        BID_COLOR
                    };
                    let y = offset_y as i32 + current_y + histogram_rect_height as i32;
                    root.draw(&Rectangle::new(
                        [
                            (offset_x as i32, offset_y as i32 + current_y),
                            (offset_x as i32 + rect_width, y),
                        ],
                        ShapeStyle::from(color).filled(),
                    ))?;
                    current_y += (histogram_rect_height + gap as u32) as i32;
                    bids_asks_y_map.insert(price.to_string(), y as f32);
                }
            }
            root.present()?;
        }

        let img_restored = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(width, height, img_rgb)
            .expect("Failed to reconstruct RGB image from raw buffer");
        *img = img_restored;
    }

    // Reset
    let mut current_y = (-{ price_rect_height_half } / 2i32);
    let offset_y =
        height as f32 / 2.0 - (bar_height * (ask_data.len() as i32) + bar_height - gap / 2) as f32;

    let offset_x = parent_offset_x;

    // Draw label
    for (price, volume) in ask_data.iter() {
        let bg_color = if price.round() == upper_bound.round() {
            BB_UPPER_BOUND_LABEL
        } else {
            TRANSPARENT_BLACK_50
        };

        if price.is_finite() && volume.is_finite() {
            let font_color = if price.round() == upper_bound.round() {
                NUM_WHITE
            } else {
                NUM_RED
            };
            draw_label(
                img,
                font,
                &format!("{price:.0}"),
                offset_x,
                offset_y + current_y as f32,
                ORDER_LABEL_SCALE,
                font_color,
                Some(bg_color),
            )?;

            draw_label(
                img,
                font,
                &format_short_number(*volume as i64).to_string(),
                (current_x + offset_x as u32) as f32,
                offset_y + current_y as f32,
                ORDER_LABEL_SCALE,
                NUM_WHITE,
                None,
            )?;

            current_y += histogram_rect_height as i32 + gap;
        }
    }

    let middle_y = (current_y + price_rect_height_half / 4) as f32 + offset_y;
    current_y += price_rect_height_half;

    for (price, volume) in bid_data.iter() {
        let bg_color = if price.round() == lower_bound.round() {
            BB_LOWER_BOUND_LABEL
        } else {
            TRANSPARENT_BLACK_50
        };

        if price.is_finite() && volume.is_finite() {
            let font_color = if price.round() == lower_bound.round() {
                NUM_WHITE
            } else {
                NUM_GREEN
            };
            draw_label(
                img,
                font,
                &format!("{price:.0}"),
                offset_x,
                offset_y + current_y as f32,
                ORDER_LABEL_SCALE,
                font_color,
                Some(bg_color),
            )?;

            draw_label(
                img,
                font,
                &format_short_number(*volume as i64).to_string(),
                (current_x as f32 + offset_x),
                offset_y + current_y as f32,
                ORDER_LABEL_SCALE,
                NUM_WHITE,
                None,
            )?;

            current_y += (histogram_rect_height + gap as u32) as i32;
        }
    }

    // Draw price line
    let price_line_y = current_price_y + price_rect_height_half as f32;
    draw_line_segment_mut(
        img,
        (
            parent_offset_x - padding_right + current_price_bounding_rect.width() as f32 + 8.0,
            price_line_y,
        ),
        (offset_x - 3.0, price_line_y),
        PRICE_LINE_COLOR,
    );
    draw_line_segment_mut(
        img,
        (offset_x - 3.0, price_line_y),
        (offset_x - 3.0, middle_y),
        PRICE_LINE_COLOR,
    );
    draw_line_segment_mut(
        img,
        (offset_x - 3.0, middle_y),
        (offset_x + 128.0, middle_y),
        PRICE_LINE_COLOR,
    );

    Ok(bids_asks_y_map)
}
