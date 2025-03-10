use crate::charts::png::encode_png;
use ab_glyph::{Font, PxScale};
use ab_glyph::{FontArc, ScaleFont};
use chrono::{DateTime, Duration};
use chrono_tz::Tz;
use common::Kline;
use image::{ImageBuffer, Rgb};
use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut, text_size};
use imageproc::rect::Rect;
use m4rs::{bolinger_band, macd, Candlestick as M4rsCandlestick};
use plotters::coord::types::RangedCoordf32;
use plotters::prelude::*;
use plotters::style::full_palette::{GREEN_100, RED_100};

use std::error::Error;

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
const AXIS_SCALE: PxScale = PxScale { x: 18.0, y: 18.0 };

// Label
const HEAD_SCALE: PxScale = PxScale { x: 22.0, y: 22.0 };
const LABEL_COLOR: Rgb<u8> = Rgb([255, 255, 255]);
const LABEL_SCALE: PxScale = PxScale { x: 20.0, y: 20.0 };
const TRANSPARENT_BLACK_50: Rgb<u8> = Rgb([0, 0, 0]); // 50% transparent black

// Styling structures
#[derive(Clone)]
pub struct PointStyle {
    pub radius: i32,
    pub color: RGBColor,
}

#[derive(Clone)]
pub struct LineStyle {
    pub stroke_width: i32,
    pub color: RGBColor,
}

#[derive(Clone)]
pub struct LabelStyle {
    pub scale: PxScale,
    pub color: Rgb<u8>,
    pub background_color: Rgb<u8>,
    pub offset_x: i32,
    pub offset_y: i32,
}

pub struct ChartMetaData {
    pub title: String,
}

// Helper functions
fn parse_kline_time(timestamp: i64, tz: &Tz) -> DateTime<Tz> {
    DateTime::from_timestamp(timestamp / 1000, 0)
        .unwrap()
        .with_timezone(tz)
}

fn kline_to_m4rs_candlestick(k: &Kline) -> M4rsCandlestick {
    M4rsCandlestick::new(
        k.open_time as u64,
        k.open_price.parse::<f64>().unwrap(),
        k.high_price.parse::<f64>().unwrap(),
        k.low_price.parse::<f64>().unwrap(),
        k.close_price.parse::<f64>().unwrap(),
        k.volume.parse::<f64>().unwrap(),
    )
}

fn parse_timeframe_duration(timeframe: &str) -> Duration {
    let (value, unit) = timeframe.split_at(timeframe.len() - 1);
    let value = value.parse::<i64>().unwrap();
    match unit {
        "m" => Duration::minutes(value),
        "h" => Duration::hours(value),
        "d" => Duration::days(value),
        _ => panic!("Unsupported timeframe unit"),
    }
}

fn get_visible_range_and_data(
    past_data: &[Kline],
    timezone: &Tz,
    candle_width: u32,
    final_width: u32,
) -> Result<(DateTime<Tz>, DateTime<Tz>, Vec<Kline>), Box<dyn Error>> {
    let total_candles = past_data.len();
    if total_candles == 0 {
        return Err("No candle data available".into());
    }

    let visible_candles = (final_width / candle_width) as usize;
    let start_index = total_candles.saturating_sub(visible_candles);

    let first_visible_time = parse_kline_time(past_data[start_index].open_time, timezone);
    let last_visible_time = parse_kline_time(past_data[total_candles - 1].open_time, timezone);

    let visible_data: Vec<Kline> = past_data
        .iter()
        .filter(|k| {
            let time = parse_kline_time(k.open_time, timezone);
            time >= first_visible_time && time <= last_visible_time
        })
        .cloned()
        .collect();

    Ok((first_visible_time, last_visible_time, visible_data))
}

fn draw_chart(
    root: DrawingArea<BitMapBackend<'_>, plotters::coord::Shift>,
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

        let mut remaining_area = bottom;
        let mut areas = Vec::new();

        if chart.volume_enabled {
            let (volume_area, rest) =
                remaining_area.split_vertically((section_height_percent).percent());
            areas.push(volume_area);
            remaining_area = rest;
        }
        println!("section_height_percent: {:?}", section_height_percent);
        if chart.macd_enabled {
            let (macd_area, rest) =
                remaining_area.split_vertically((section_height_percent * 2).percent());
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

fn draw_axis_labels(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    font: &impl Font,
    past_data: &[Kline],
    chart: &Chart,
    height: u32,
    final_width: u32,
    margin_right: u32,
    min_price: f32,
    max_price: f32,
) -> Result<(), Box<dyn Error>> {
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

    Ok(())
}

fn draw_lines(
    root: &DrawingArea<BitMapBackend<'_>, Cartesian2d<RangedCoordf32, RangedCoordf32>>,
    chart: &Chart,
) -> Result<(), Box<dyn Error>> {
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

fn draw_labels(
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

fn draw_label<F: Font>(
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

    draw_filled_rect_mut(
        img,
        Rect::at(x, y).of_size(text_width, text_height),
        background_color,
    );

    draw_text_mut(
        img,
        color,
        x,
        y + (font_metrics.descent() as i32),
        scale,
        font,
        text,
    );

    Ok(())
}

// Chart struct
pub struct Chart {
    timezone: Tz,
    timeframe: String,
    past_candle_data: Option<Vec<Kline>>,
    metadata: ChartMetaData,
    font_data: Option<Vec<u8>>,
    candle_width: u32,
    points: Vec<(f32, f32)>,
    point_style: Option<PointStyle>,
    lines: Vec<[(f32, f32); 2]>,
    line_style: Option<LineStyle>,
    labels: Vec<(f32, f32, String)>,
    label_style: Option<LabelStyle>,
    macd_enabled: bool,
    bollinger_enabled: bool,
    volume_enabled: bool,
    stoch_rsi_enabled: bool, // New field
}

impl Chart {
    pub fn new(timeframe: &str, timezone: Tz) -> Self {
        Chart {
            timezone,
            timeframe: timeframe.to_string(),
            past_candle_data: None,
            metadata: ChartMetaData {
                title: String::new(),
            },
            font_data: None,
            candle_width: 10,
            points: Vec::new(),
            point_style: None,
            lines: Vec::new(),
            line_style: None,
            labels: Vec::new(),
            label_style: None,
            macd_enabled: false,
            bollinger_enabled: false,
            volume_enabled: false,
            stoch_rsi_enabled: false,
        }
    }

    pub fn with_past_candle(mut self, past_candle_data: Vec<Kline>) -> Self {
        self.past_candle_data = Some(past_candle_data);
        self
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.metadata.title = title.to_string();
        self
    }
    pub fn with_font_data(mut self, font_data: Vec<u8>) -> Self {
        self.font_data = Some(font_data);
        self
    }

    #[allow(dead_code)]
    pub fn with_candle_width(mut self, width: u32) -> Self {
        self.candle_width = width;
        self
    }

    #[allow(dead_code)]
    pub fn with_points(mut self, points: Vec<(f32, f32)>) -> Self {
        self.points = points;
        self
    }

    #[allow(dead_code)]
    pub fn with_point_style(mut self, radius: i32, color: RGBColor) -> Self {
        self.point_style = Some(PointStyle { radius, color });
        self
    }

    #[allow(dead_code)]
    pub fn with_lines(mut self, lines: Vec<[(f32, f32); 2]>) -> Self {
        self.lines = lines;
        self
    }

    #[allow(dead_code)]
    pub fn with_line_style(mut self, stroke_width: i32, color: RGBColor) -> Self {
        self.line_style = Some(LineStyle {
            stroke_width,
            color,
        });
        self
    }

    #[allow(dead_code)]
    pub fn with_labels(mut self, labels: Vec<(f32, f32, String)>) -> Self {
        self.labels = labels;
        self
    }

    #[allow(dead_code)]
    pub fn with_label_style(
        mut self,
        scale_x: f32,
        scale_y: f32,
        color: Rgb<u8>,
        background_color: Rgb<u8>,
        offset_x: i32,
        offset_y: i32,
    ) -> Self {
        self.label_style = Some(LabelStyle {
            scale: PxScale {
                x: scale_x,
                y: scale_y,
            },
            color,
            background_color,
            offset_x,
            offset_y,
        });
        self
    }

    pub fn with_macd(mut self) -> Self {
        self.macd_enabled = true;
        self
    }

    pub fn with_bollinger_band(mut self) -> Self {
        self.bollinger_enabled = true;
        self
    }

    #[allow(dead_code)]
    pub fn with_volume(mut self) -> Self {
        self.volume_enabled = true;
        self
    }

    #[allow(dead_code)]
    pub fn with_stoch_rsi(mut self) -> Self {
        self.stoch_rsi_enabled = true;
        self
    }

    pub fn build(self) -> Result<Vec<u8>, Box<dyn Error>> {
        if self.past_candle_data.is_none() {
            return Err("Candle data set is required".into());
        };

        let font_data = self
            .font_data
            .as_ref()
            .ok_or("Font data is required")?
            .clone();
        let font = FontArc::try_from_vec(font_data)?;
        let timezone = &self.timezone;

        let all_candle_data = &self.past_candle_data.clone().unwrap();
        let past_data = self.past_candle_data.as_deref().unwrap_or(&[]);

        let margin_right = 100;
        let total_candles = all_candle_data.len();
        let candle_width = self.candle_width;
        let total_width = total_candles as u32 * candle_width;
        let final_width = 768;
        let height = 1024;

        let plot_width = total_width.max(final_width);
        let bar: (u32, u32) = (plot_width, height);
        let mut buffer = vec![0; (plot_width * height * 3) as usize];

        let first_time = parse_kline_time(all_candle_data[0].open_time, timezone);
        let last_time = parse_kline_time(
            all_candle_data[all_candle_data.len() - 1].open_time,
            timezone,
        );

        let prices: Vec<f32> = all_candle_data
            .iter()
            .flat_map(|k| {
                vec![
                    k.open_price.parse::<f32>().unwrap(),
                    k.high_price.parse::<f32>().unwrap(),
                    k.low_price.parse::<f32>().unwrap(),
                    k.close_price.parse::<f32>().unwrap(),
                ]
            })
            .collect();
        let min_price = prices.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let max_price = prices.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));

        {
            let root = BitMapBackend::with_buffer(&mut buffer, bar).into_drawing_area();
            draw_chart(
                root,
                all_candle_data,
                past_data,
                timezone,
                &self,
                min_price,
                max_price,
                first_time,
                last_time,
                margin_right,
                candle_width,
                final_width,
            )?;
        }

        let mut imgbuf: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(plot_width, height);
        imgbuf.copy_from_slice(buffer.as_slice());

        let crop_x = plot_width.saturating_sub(final_width);
        let mut cropped_img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            image::imageops::crop_imm(&imgbuf, crop_x, 0, final_width, height).to_image();

        // Add header details on the cropped image
        draw_candle_detail(&mut cropped_img, &self, &font)?;
        if self.bollinger_enabled {
            draw_bollinger_detail(&mut cropped_img, past_data, &font)?;
        }

        if self.volume_enabled || self.macd_enabled || self.stoch_rsi_enabled {
            let num_indicators = [
                self.volume_enabled,
                self.macd_enabled,
                self.stoch_rsi_enabled,
            ]
            .iter()
            .filter(|&&enabled| enabled)
            .count() as f32;

            let section_height = height as f32 * 0.5 / num_indicators;
            let top_section_height = height as f32 * 0.5;

            let mut current_y = top_section_height as i32;

            if self.volume_enabled {
                draw_volume_detail(&mut cropped_img, past_data, &font, current_y)?;
                current_y += section_height as i32;
            }
            if self.macd_enabled {
                draw_macd_detail(&mut cropped_img, past_data, &font, current_y)?;
                current_y += section_height as i32;
            }
            if self.stoch_rsi_enabled {
                draw_stoch_rsi_detail(&mut cropped_img, past_data, &font, current_y)?;
            }
        }

        {
            let root = BitMapBackend::with_buffer(&mut cropped_img, (final_width, height))
                .into_drawing_area();
            let root = root.apply_coord_spec(Cartesian2d::<RangedCoordf32, RangedCoordf32>::new(
                0f32..1f32,
                0f32..1f32,
                (0..final_width as i32, 0..height as i32),
            ));

            draw_lines(&root, &self)?;
        }

        draw_axis_labels(
            &mut cropped_img,
            &font.clone(),
            past_data,
            &self,
            height,
            final_width,
            margin_right,
            min_price,
            max_price,
        )?;

        draw_labels(&mut cropped_img, &font, &self, final_width, height)?;

        Ok(encode_png(&cropped_img)?)
    }
}

// Drawing helpers
fn draw_candlesticks<F>(
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

fn draw_candle_detail(
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

fn draw_bollinger_bands(
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

fn draw_bollinger_detail(
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

fn draw_volume_bars(
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

fn draw_volume_detail(
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

fn draw_macd(
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

fn draw_macd_detail(
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

fn draw_stoch_rsi_detail(
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
            "Stock RSI 14 14 3 {:.2} {:.2}",
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

#[cfg(test)]
mod test {
    use super::*;
    use chrono_tz::Asia::Tokyo;
    use common::binance::fetch_binance_kline_data;

    #[tokio::test]
    async fn entry_point() {
        let pair_symbol = "SOL_USDT";
        let timeframe = "1h";
        let font_data = include_bytes!("../../RobotoMono-Regular.ttf").to_vec();

        let limit = 24 * 10;
        let candle_data = fetch_binance_kline_data::<Kline>(pair_symbol, timeframe, limit)
            .await
            .unwrap();

        let png = Chart::new(timeframe, Tokyo)
            .with_candle_width(6)
            .with_past_candle(candle_data)
            .with_title(pair_symbol)
            .with_font_data(font_data)
            .with_volume()
            .with_macd()
            .with_stoch_rsi()
            .with_bollinger_band()
            // .with_labels(vec![(0.75, 0.25, "71% BULL".to_string())])
            // .with_label_style(20.0, 20.0, Rgb([0, 0, 255]), Rgb([0, 255, 255]), 10, 5)
            .build()
            .unwrap();

        std::fs::write("test.png", png).unwrap();
    }
}
