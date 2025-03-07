use crate::charts::png::encode_png;
use ab_glyph::ScaleFont;
use ab_glyph::{Font, FontRef, PxScale};
use chrono::DateTime;
use chrono_tz::Tz;
use common::Kline;
use image::{ImageBuffer, Rgb};
use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut};
use imageproc::rect::Rect;
use m4rs::BollingerBandEntry;
use m4rs::{bolinger_band, macd, Candlestick as M4rsCandlestick};
use plotters::coord::types::RangedCoordf32;
use plotters::prelude::full_palette::PURPLE;
use plotters::prelude::*;
use rand::Rng;
use std::error::Error;

// Styling structures (unchanged)
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

fn draw_candlesticks<F>(
    chart: &mut ChartContext<
        '_,
        BitMapBackend<'_>,
        Cartesian2d<RangedDateTime<DateTime<Tz>>, RangedCoordf32>,
    >,
    candle_data: &[Kline],
    timezone: &Tz,
    bar_width: u32,
    color_selector: F,
) -> Result<(), Box<dyn Error>>
where
    F: Fn(bool) -> RGBAColor,
{
    chart.draw_series(candle_data.iter().map(|k| {
        let open = k.open_price.parse::<f32>().unwrap();
        let high = k.high_price.parse::<f32>().unwrap();
        let low = k.low_price.parse::<f32>().unwrap();
        let close = k.close_price.parse::<f32>().unwrap();
        let is_bullish = close >= open;
        let color = color_selector(is_bullish);
        CandleStick::new(
            parse_kline_time(k.open_time, timezone),
            open,
            high,
            low,
            close,
            ShapeStyle::from(&color).filled(),
            ShapeStyle::from(&color).filled(),
            bar_width,
        )
    }))?;
    Ok(())
}

// Chart struct with added technical analysis methods
pub struct Chart {
    timezone: Tz,
    past_candle_data: Option<Vec<Kline>>,
    predicted_candle_data: Option<Vec<Kline>>,
    metadata: ChartMetaData,
    font_data: Option<Vec<u8>>,
    candle_width: u32,
    candle_height: u32,
    points: Vec<(f32, f32)>,
    point_style: Option<PointStyle>,
    lines: Vec<[(f32, f32); 2]>,
    line_style: Option<LineStyle>,
    labels: Vec<(f32, f32, String)>,
    label_style: Option<LabelStyle>,
    macd_enabled: bool,
    bollinger_enabled: bool,
}

impl Chart {
    pub fn new(timezone: Tz) -> Self {
        Chart {
            timezone,
            past_candle_data: None,
            predicted_candle_data: None,
            metadata: ChartMetaData {
                title: String::new(),
            },
            font_data: None,
            candle_width: 10,
            candle_height: 5,
            points: Vec::new(),
            point_style: None,
            lines: Vec::new(),
            line_style: None,
            labels: Vec::new(),
            label_style: None,
            macd_enabled: false,
            bollinger_enabled: false,
        }
    }

    // Existing builder methods (unchanged except where noted)
    pub fn with_past_candle(mut self, past_candle_data: Vec<Kline>) -> Self {
        self.past_candle_data = Some(past_candle_data);
        self
    }

    pub fn with_predicted_candle(mut self, predicted_candle_data: Vec<Kline>) -> Self {
        self.predicted_candle_data = Some(predicted_candle_data);
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

    pub fn with_candle_dimensions(mut self, width: u32, height: u32) -> Self {
        self.candle_width = width;
        self.candle_height = height;
        self
    }

    pub fn with_points(mut self, points: Vec<(f32, f32)>) -> Self {
        self.points = points;
        self
    }

    pub fn with_point_style(mut self, radius: i32, color: RGBColor) -> Self {
        self.point_style = Some(PointStyle { radius, color });
        self
    }

    pub fn with_lines(mut self, lines: Vec<[(f32, f32); 2]>) -> Self {
        self.lines = lines;
        self
    }

    pub fn with_line_style(mut self, stroke_width: i32, color: RGBColor) -> Self {
        self.line_style = Some(LineStyle {
            stroke_width,
            color,
        });
        self
    }

    pub fn with_labels(mut self, labels: Vec<(f32, f32, String)>) -> Self {
        self.labels = labels;
        self
    }

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

    // New methods for technical analysis
    pub fn with_macd(mut self) -> Self {
        self.macd_enabled = true;
        self
    }

    pub fn with_bollinger_band(mut self) -> Self {
        self.bollinger_enabled = true;
        self
    }

    pub fn build(self) -> Result<Vec<u8>, Box<dyn Error>> {
        if self.past_candle_data.is_none() && self.predicted_candle_data.is_none() {
            return Err("At least one candle data set is required".into());
        }
        let font_data = self.font_data.ok_or("Font data is required")?;
        let font = FontRef::try_from_slice(&font_data)?;
        let timezone = &self.timezone;

        let all_candle_data = match (&self.past_candle_data, &self.predicted_candle_data) {
            (Some(past), Some(pred)) => [past.clone(), pred.clone()].concat(),
            (Some(past), None) => past.clone(),
            (None, Some(pred)) => pred.clone(),
            (None, None) => unreachable!(),
        };

        let num_candles = all_candle_data.len() as u32;
        let calculated_width = num_candles * self.candle_width;
        let calculated_height = num_candles * self.candle_height + 200;
        const MIN_DIMENSION: u32 = 768;
        let width = calculated_width.max(MIN_DIMENSION);
        let height = calculated_height.max(MIN_DIMENSION);

        let bar: (u32, u32) = (width, height);
        let mut buffer = vec![0; (width * height * 3) as usize];

        {
            let root = BitMapBackend::with_buffer(&mut buffer, bar).into_drawing_area();
            root.fill(&BLACK)?;

            // Split into two rows: top (75%), bottom (25%)
            let (top, bottom) = root.split_vertically((75).percent());

            // Top row: Candlesticks
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

            let first_time = parse_kline_time(all_candle_data[0].open_time, timezone);
            let last_time = parse_kline_time(
                all_candle_data[all_candle_data.len() - 1].open_time,
                timezone,
            );
            let mid_time = first_time + (last_time - first_time) / 2;

            let mut top_chart = ChartBuilder::on(&top)
                .margin(20)
                .build_cartesian_2d(first_time..last_time, min_price * 0.95..max_price * 1.05)?;

            top_chart
                .configure_mesh()
                .light_line_style(RGBColor(48, 48, 48))
                .draw()?;

            let bar_width = (width as f32 / num_candles as f32).max(1.0) as u32;

            // Draw past candlesticks (left 50%)
            if let Some(past_candle_data) = &self.past_candle_data {
                draw_candlesticks(
                    &mut top_chart,
                    past_candle_data,
                    timezone,
                    bar_width,
                    |is_bullish| {
                        if is_bullish {
                            GREEN.into()
                        } else {
                            RED.into()
                        }
                    },
                )?;
            }

            // Draw predicted candlesticks (right 50%, 25% transparent)
            if let Some(predicted_candle_data) = &self.predicted_candle_data {
                draw_candlesticks(
                    &mut top_chart,
                    predicted_candle_data,
                    timezone,
                    bar_width,
                    |is_bullish| {
                        if is_bullish {
                            RGBAColor(0, 255, 0, 0.25)
                        } else {
                            RGBAColor(255, 0, 0, 0.25)
                        }
                    },
                )?;
            }

            // Draw Bollinger Bands if enabled
            if self.bollinger_enabled {
                if let Some(past_data) = &self.past_candle_data {
                    let past_m4rs_candles: Vec<M4rsCandlestick> =
                        past_data.iter().map(kline_to_m4rs_candlestick).collect();
                    let bb_result = bolinger_band(&past_m4rs_candles, 20)?; // 20-period SMA
                    let bb_lines: Vec<(DateTime<Tz>, f32, f32, f32)> = bb_result
                        .iter()
                        .map(|entry| {
                            let t = parse_kline_time(entry.at as i64, timezone);
                            let avg = entry.avg as f32; // Middle band (SMA)
                            let upper = (entry.avg + 2.0 * entry.sigma) as f32; // Upper band (2 SD)
                            let lower = (entry.avg - 2.0 * entry.sigma) as f32; // Lower band (2 SD)
                            (t, avg, upper, lower)
                        })
                        .collect();

                    // Plot each band with proper scaling
                    top_chart.draw_series(LineSeries::new(
                        bb_lines.iter().map(|(t, avg, _, _)| (*t, *avg)),
                        &PURPLE,
                    ))?;
                    top_chart.draw_series(LineSeries::new(
                        bb_lines.iter().map(|(t, _, upper, _)| (*t, *upper)),
                        &PURPLE,
                    ))?;
                    top_chart.draw_series(LineSeries::new(
                        bb_lines.iter().map(|(t, _, _, lower)| (*t, *lower)),
                        &PURPLE,
                    ))?;
                }
            }
            if let (Some(past_data), Some(predicted_data)) =
                (&self.past_candle_data, &self.predicted_candle_data)
            {
                let overlap_candles = (past_data.len() as f32 * 0.1).ceil() as usize;
                let overlap_start = past_data.len() - overlap_candles; // Start at index 60 for past_data[60..72]
                let past_overlap = &past_data[overlap_start..];
                let predicted_overlap = &predicted_data[0..overlap_candles]; // predicted_data[0..12]

                let differences: Vec<(f32, f32)> = past_overlap
                    .iter()
                    .zip(predicted_overlap.iter())
                    .map(|(past, pred)| {
                        let past_high = past.high_price.parse::<f32>().unwrap();
                        let pred_high = pred.high_price.parse::<f32>().unwrap();
                        let past_low = past.low_price.parse::<f32>().unwrap();
                        let pred_low = pred.low_price.parse::<f32>().unwrap();
                        (pred_high - past_high, pred_low - past_low)
                    })
                    .collect();

                let diff_values: Vec<f32> =
                    differences.iter().flat_map(|(h, l)| vec![*h, *l]).collect();
                let min_diff = diff_values.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                let max_diff = diff_values.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                let y_min = min_diff.min(0.0) * 1.1;
                let y_max = max_diff.max(0.0) * 1.1;

                let mut bottom_chart = ChartBuilder::on(&bottom)
                    .margin(20)
                    .build_cartesian_2d(first_time..last_time, y_min..y_max)?;

                bottom_chart
                    .configure_mesh()
                    .light_line_style(RGBColor(48, 48, 48))
                    .draw()?;

                // Midline at y=0
                let mid_y = 0.0;
                bottom_chart
                    .draw_series(LineSeries::new(
                        vec![(first_time, mid_y), (last_time, mid_y)],
                        &RED,
                    ))?
                    .label("Midline")
                    .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));

                // Yellow bars for high differences
                bottom_chart.draw_series(past_overlap.iter().zip(differences.iter()).map(
                    |(past, (high_diff, _))| {
                        let time = parse_kline_time(past.open_time, timezone);
                        let bar_width = chrono::Duration::minutes(1);
                        Rectangle::new(
                            [
                                (time - bar_width / 2, mid_y),
                                (time + bar_width / 2, *high_diff),
                            ],
                            ShapeStyle::from(&YELLOW).filled(),
                        )
                    },
                ))?;

                // Blue bars for low differences
                bottom_chart.draw_series(past_overlap.iter().zip(differences.iter()).map(
                    |(past, (_, low_diff))| {
                        let time = parse_kline_time(past.open_time, timezone);
                        let bar_width = chrono::Duration::minutes(1);
                        Rectangle::new(
                            [
                                (time - bar_width / 2, mid_y),
                                (time + bar_width / 2, *low_diff),
                            ],
                            ShapeStyle::from(&BLUE).filled(),
                        )
                    },
                ))?;
            }
        }

        // Image buffer and additional drawings (points, lines, labels, title)
        let mut imgbuf: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(width, height);
        imgbuf.copy_from_slice(buffer.as_slice());

        let white = Rgb([255u8, 255u8, 255u8]);
        {
            let root = BitMapBackend::with_buffer(&mut imgbuf, bar).into_drawing_area();
            let root = root.apply_coord_spec(Cartesian2d::<RangedCoordf32, RangedCoordf32>::new(
                0f32..1f32,
                0f32..1f32,
                (0..width as i32, 0..height as i32),
            ));

            if !self.lines.is_empty() {
                let style = self.line_style.clone().unwrap_or(LineStyle {
                    stroke_width: 2,
                    color: YELLOW,
                });
                for &[p1, p2] in self.lines.iter() {
                    root.draw(&PathElement::new(
                        vec![p1, p2],
                        ShapeStyle::from(&style.color).stroke_width(style.stroke_width as u32),
                    ))?;
                }
            }

            if !self.points.is_empty() {
                let style = self.point_style.clone().unwrap_or(PointStyle {
                    radius: 3,
                    color: WHITE,
                });
                for &(x, y) in self.points.iter() {
                    root.draw(&Circle::new(
                        (x, y),
                        style.radius,
                        ShapeStyle::from(&style.color).filled(),
                    ))?;
                }
            }
        }

        if !self.labels.is_empty() {
            let style = self.label_style.clone().unwrap_or(LabelStyle {
                scale: PxScale { x: 15.0, y: 15.0 },
                color: white,
                background_color: Rgb([0, 0, 0]),
                offset_x: 5,
                offset_y: 0,
            });
            let font_metrics = font.as_scaled(style.scale);
            for (x, y, text) in self.labels.iter() {
                let mut total_width = 0.0f32;
                for c in text.chars() {
                    let glyph_id = font_metrics.glyph_id(c);
                    let glyph = ab_glyph::Glyph {
                        id: glyph_id,
                        scale: style.scale,
                        position: ab_glyph::Point { x: 0.0, y: 0.0 },
                    };
                    total_width += font_metrics.glyph_bounds(&glyph).width();
                }
                let text_width = total_width.ceil() as i32;
                let text_height = (font_metrics.ascent() - font_metrics.descent()).ceil() as i32;
                let x_pos = (*x * width as f32) as i32 + style.offset_x;
                let y_pos = (*y * height as f32) as i32 + style.offset_y - text_height;

                draw_filled_rect_mut(
                    &mut imgbuf,
                    Rect::at(x_pos - 4, y_pos - 4)
                        .of_size((text_width + 6) as u32, (text_height + 1) as u32),
                    style.background_color,
                );

                draw_text_mut(
                    &mut imgbuf,
                    style.color,
                    x_pos,
                    y_pos + (font_metrics.descent() as i32),
                    style.scale,
                    &font,
                    text,
                );
            }
        }

        draw_text_mut(
            &mut imgbuf,
            white,
            10,
            10,
            PxScale { x: 50.0, y: 50.0 },
            &font,
            &self.metadata.title,
        );

        Ok(encode_png(&imgbuf)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use chrono_tz::Asia::Tokyo;
    use common::binance::fetch_binance_kline_data;
    use image::Rgb;
    use plotters::style::RGBColor;
    use rand::Rng;

    fn tweak_candle_data(candle: &Kline) -> Kline {
        let mut rng = rand::thread_rng();
        let tweak_factor = 0.005; // Reduced from 0.01 to 0.005 for less randomness
        Kline {
            open_time: candle.open_time,
            open_price: (candle.open_price.parse::<f32>().unwrap()
                * (1.0 + rng.gen_range(-tweak_factor..tweak_factor)))
            .to_string(),
            high_price: (candle.high_price.parse::<f32>().unwrap()
                * (1.0 + rng.gen_range(-tweak_factor..tweak_factor)))
            .to_string(),
            low_price: (candle.low_price.parse::<f32>().unwrap()
                * (1.0 + rng.gen_range(-tweak_factor..tweak_factor)))
            .to_string(),
            close_price: (candle.close_price.parse::<f32>().unwrap()
                * (1.0 + rng.gen_range(-tweak_factor..tweak_factor)))
            .to_string(),
            volume: candle.volume.clone(),
            close_time: candle.close_time,
            quote_asset_volume: candle.quote_asset_volume.clone(),
            number_of_trades: candle.number_of_trades,
            taker_buy_base_asset_volume: candle.taker_buy_base_asset_volume.clone(),
            taker_buy_quote_asset_volume: candle.taker_buy_quote_asset_volume.clone(),
            ignore: candle.ignore.clone(),
        }
    }

    #[tokio::test]
    async fn entry_point() {
        let candle_data = fetch_binance_kline_data::<Kline>("SOL_USDT", "1m", 120)
            .await
            .unwrap();

        let total_candles = candle_data.len();
        let past_candles = (total_candles as f32 * 0.5).ceil() as usize;
        let overlap_candles = (total_candles as f32 * 0.1).ceil() as usize;
        let predicted_start = total_candles - (total_candles as f32 * 0.5).ceil() as usize;

        let past_data = candle_data[..past_candles + overlap_candles].to_vec();
        let predicted_candle_data: Vec<Kline> = candle_data[predicted_start..]
            .iter()
            .map(|k| tweak_candle_data(k))
            .collect();

        let font_data = include_bytes!("../../Roboto-Light.ttf").to_vec();

        let png = Chart::new(Tokyo)
            .with_past_candle(past_data)
            .with_predicted_candle(predicted_candle_data)
            .with_title("SOL/USDT")
            .with_font_data(font_data)
            .with_candle_dimensions(10, 5)
            .with_macd()
            .with_bollinger_band()
            .with_labels(vec![(0.75, 0.25, "71% BULL".to_string())])
            .with_label_style(20.0, 20.0, Rgb([0, 0, 255]), Rgb([0, 255, 255]), 10, 5)
            .build()
            .unwrap();

        std::fs::write("test.png", png).unwrap();
    }
}
