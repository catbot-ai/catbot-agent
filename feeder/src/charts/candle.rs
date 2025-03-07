use crate::charts::png::encode_png;
use ab_glyph::ScaleFont;
use ab_glyph::{Font, FontRef, PxScale};
use chrono::DateTime;
use chrono_tz::Tz;
use common::Kline;
use image::{ImageBuffer, Rgb};
use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut};
use imageproc::rect::Rect;
use plotters::coord::types::RangedCoordf32;
use plotters::prelude::*;
use std::error::Error;

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

// Convert Kline timestamp (i64) to DateTime<Tz>
fn parse_kline_time(timestamp: i64, tz: &Tz) -> DateTime<Tz> {
    DateTime::from_timestamp(timestamp / 1000, 0)
        .unwrap()
        .with_timezone(tz)
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
    F: Fn(bool) -> RGBAColor, // Closure returns a color based on is_bullish (close >= open)
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

// Builder pattern for Chart
pub struct Chart {
    timezone: Tz,
    past_candle_data: Option<Vec<Kline>>,
    predicted_candle_data: Option<Vec<Kline>>,
    metadata: ChartMetaData,
    font_data: Option<Vec<u8>>,
    candle_width: u32,
    candle_height: u32,
    points: Vec<(f32, f32)>, // (x, y) coordinates (0.0 to 1.0)
    point_style: Option<PointStyle>,
    lines: Vec<[(f32, f32); 2]>, // Pairs of (x1, y1), (x2, y2) coordinates (0.0 to 1.0)
    line_style: Option<LineStyle>,
    labels: Vec<(f32, f32, String)>, // (x, y, text) coordinates (0.0 to 1.0) and text
    label_style: Option<LabelStyle>,
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
        }
    }

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

    pub fn build(self) -> Result<Vec<u8>, Box<dyn Error>> {
        // Validate required fields (at least one candle dataset)
        if self.past_candle_data.is_none() && self.predicted_candle_data.is_none() {
            return Err("At least one candle data set is required".into());
        }
        let font_data = self.font_data.ok_or("Font data is required")?;
        let font = FontRef::try_from_slice(&font_data)?;
        let timezone = &self.timezone;

        // Combine all candle data for calculating ranges
        let all_candle_data = match (&self.past_candle_data, &self.predicted_candle_data) {
            (Some(past), Some(pred)) => [past.clone(), pred.clone()].concat(),
            (Some(past), None) => past.clone(),
            (None, Some(pred)) => pred.clone(),
            (None, None) => unreachable!(), // Checked above
        };

        // Calculate dimensions
        let num_candles = all_candle_data.len() as u32;
        let calculated_width = num_candles * self.candle_width;
        let calculated_height = num_candles * self.candle_height + 200;
        const MIN_DIMENSION: u32 = 768;
        let width = calculated_width.max(MIN_DIMENSION);
        let height = calculated_height.max(MIN_DIMENSION);

        let bar: (u32, u32) = (width, height);
        let mut buffer = vec![0; (width * height * 3) as usize];

        // Draw chart
        {
            let root = BitMapBackend::with_buffer(&mut buffer, bar).into_drawing_area();
            root.fill(&BLACK)?;

            // Calculate x-axis range
            let first_time = parse_kline_time(all_candle_data[0].open_time, timezone);
            let last_time = parse_kline_time(
                all_candle_data[all_candle_data.len() - 1].open_time,
                timezone,
            );

            // Calculate y-axis range
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

            let mut chart = ChartBuilder::on(&root)
                .margin(20)
                .build_cartesian_2d(first_time..last_time, min_price * 0.95..max_price * 1.05)?;

            chart
                .configure_mesh()
                .light_line_style(RGBColor(48, 48, 48))
                .draw()?;

            let bar_width = (width as f32 / num_candles as f32).max(1.0) as u32;

            // Draw past candlesticks with solid colors
            if let Some(past_candle_data) = &self.past_candle_data {
                draw_candlesticks(
                    &mut chart,
                    past_candle_data,
                    timezone,
                    bar_width,
                    |is_bullish| {
                        if is_bullish {
                            GREEN.into() // Convert RGBColor to RGBAColor (opaque)
                        } else {
                            RED.into()
                        }
                    },
                )?;
            }

            // Draw predicted candlesticks with 50% transparency
            if let Some(predicted_candle_data) = &self.predicted_candle_data {
                draw_candlesticks(
                    &mut chart,
                    predicted_candle_data,
                    timezone,
                    bar_width,
                    |is_bullish| {
                        if is_bullish {
                            RGBAColor(0, 255, 0, 0.5) // Green with 50% opacity
                        } else {
                            RGBAColor(255, 0, 0, 0.5) // Red with 50% opacity
                        }
                    },
                )?;
            }

            // Draw vertical line at the end of past data if both datasets exist
            if self.past_candle_data.is_some() && self.predicted_candle_data.is_some() {
                let past_end_time = parse_kline_time(
                    self.past_candle_data
                        .as_ref()
                        .unwrap()
                        .last()
                        .unwrap()
                        .open_time,
                    timezone,
                );
                chart.draw_series(LineSeries::new(
                    vec![
                        (past_end_time, min_price * 0.95),
                        (past_end_time, max_price * 1.05),
                    ],
                    &WHITE,
                ))?;
            }
        }

        // Create image buffer and copy data
        let mut imgbuf: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(width, height);
        imgbuf.copy_from_slice(buffer.as_slice());

        // Draw points, lines, labels, and title (unchanged from original)
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
                    let bounds = font_metrics.glyph_bounds(&glyph);
                    total_width += bounds.width()
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
    use common::Kline;
    use image::Rgb;
    use plotters::style::RGBColor;
    use rand::Rng;

    use crate::charts::candle::Chart;

    fn tweak_candle_data(candle: &Kline) -> Kline {
        let mut rng = rand::rng();
        let tweak_factor = 0.01; // 1% tweak
        Kline {
            open_time: candle.open_time,
            open_price: (candle.open_price.parse::<f32>().unwrap()
                * (1.0 + rng.random_range(-tweak_factor..tweak_factor)))
            .to_string(),
            high_price: (candle.high_price.parse::<f32>().unwrap()
                * (1.0 + rng.random_range(-tweak_factor..tweak_factor)))
            .to_string(),
            low_price: (candle.low_price.parse::<f32>().unwrap()
                * (1.0 + rng.random_range(-tweak_factor..tweak_factor)))
            .to_string(),
            close_price: (candle.close_price.parse::<f32>().unwrap()
                * (1.0 + rng.random_range(-tweak_factor..tweak_factor)))
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
        use chrono_tz::Asia::Tokyo;
        use common::binance::fetch_binance_kline_data;

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
            // .with_lines(vec![[(0.5, 0.6), (0.25, 0.33)], [(0.8, 0.8), (0.5, 0.6)]])
            // .with_line_style(3, RGBColor(0, 128, 0))
            // .with_points(vec![(0.5, 0.6), (0.25, 0.33), (0.8, 0.8)])
            // .with_point_style(5, RGBColor(0, 255, 0))
            .with_labels(vec![(0.75, 0.35, "71% BULL".to_string())])
            .with_label_style(20.0, 20.0, Rgb([0, 0, 255]), Rgb([0, 255, 255]), 10, 5)
            .build()
            .unwrap();

        std::fs::write("test.png", png).unwrap();
    }
}
