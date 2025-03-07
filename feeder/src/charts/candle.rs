use crate::charts::png::encode_png;
use ab_glyph::{FontRef, PxScale};
use chrono::{DateTime, Duration};
use chrono_tz::Tz;
use common::Kline;
use image::ImageBuffer;
use image::Rgb;
use imageproc::drawing::draw_text_mut;
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

// Builder pattern for Chart
pub struct Chart {
    timezone: Tz,
    candle_data: Option<Vec<Kline>>,
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
            candle_data: None,
            metadata: ChartMetaData {
                title: String::new(),
            },
            font_data: None,
            candle_width: 5,
            candle_height: 5,
            points: Vec::new(),
            point_style: None,
            lines: Vec::new(),
            line_style: None,
            labels: Vec::new(),
            label_style: None,
        }
    }

    pub fn with_candle(mut self, candle_data: Vec<Kline>) -> Self {
        self.candle_data = Some(candle_data);
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
        offset_x: i32,
        offset_y: i32,
    ) -> Self {
        self.label_style = Some(LabelStyle {
            scale: PxScale {
                x: scale_x,
                y: scale_y,
            },
            color,
            offset_x,
            offset_y,
        });
        self
    }

    pub fn build(self) -> Result<Vec<u8>, Box<dyn Error>> {
        // Validate required fields
        let candle_data = self.candle_data.ok_or("Candle data is required")?;
        let font_data = self.font_data.ok_or("Font data is required")?;
        let font = FontRef::try_from_slice(&font_data)?;
        let timezone = &self.timezone;

        // Calculate WIDTH and HEIGHT based on candle count and dimensions
        let num_candles = candle_data.len() as u32;
        let calculated_width = num_candles * self.candle_width;
        let calculated_height = num_candles * self.candle_height + 200; // Add extra height for labels and title

        // Ensure minimum dimensions of 768x768
        const MIN_DIMENSION: u32 = 768;
        let width = calculated_width.max(MIN_DIMENSION);
        let height = calculated_height.max(MIN_DIMENSION);

        // Raw
        let bar: (u32, u32) = (width, height);
        let baz: usize = (width * height * 3) as usize;
        let mut buffer = vec![0; baz];

        // Draw chart
        {
            let root = BitMapBackend::with_buffer(&mut buffer, bar).into_drawing_area();
            root.fill(&BLACK)?;

            // Calculate x-axis range without padding
            let first_time = parse_kline_time(candle_data[0].open_time, timezone);
            let last_time =
                parse_kline_time(candle_data[candle_data.len() - 1].open_time, timezone);

            // Determine min and max prices for the Y-axis range
            let prices: Vec<f32> = candle_data
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

            let mut chart = ChartBuilder::on(&root).margin(20).build_cartesian_2d(
                first_time..last_time,
                min_price * 0.95..max_price * 1.05, // Add some padding to the Y-axis
            )?;

            chart
                .configure_mesh()
                .x_labels(10) // Show up to 10 labels
                .x_label_formatter(&|x| x.format("%Y-%m-%d %H:%M").to_string()) // Format x-axis labels
                .light_line_style(RGBColor(48, 48, 48))
                .draw()?;

            // Calculate bar width to fit all candles without gaps
            let bar_width = (width as f32 / num_candles as f32).max(1.0) as u32;

            chart.draw_series(candle_data.iter().map(|k| {
                let open = k.open_price.parse::<f32>().unwrap();
                let high = k.high_price.parse::<f32>().unwrap();
                let low = k.low_price.parse::<f32>().unwrap();
                let close = k.close_price.parse::<f32>().unwrap();
                let color = if close >= open { GREEN } else { RED };
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
        }

        // Create image buffer
        let mut imgbuf: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(width, height);
        imgbuf.copy_from_slice(buffer.as_slice());

        // Colors
        let white = Rgb([255u8, 255u8, 255u8]);

        // Draw points and lines using normalized coordinates
        {
            let root = BitMapBackend::with_buffer(&mut imgbuf, bar).into_drawing_area();
            let root = root.apply_coord_spec(Cartesian2d::<RangedCoordf32, RangedCoordf32>::new(
                0f32..1f32,
                0f32..1f32,
                (0..width as i32, 0..height as i32),
            ));

            // Draw points
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

            // Draw lines
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
        }

        // Draw labels
        if !self.labels.is_empty() {
            let style = self.label_style.clone().unwrap_or(LabelStyle {
                scale: PxScale { x: 15.0, y: 15.0 },
                color: white,
                offset_x: 5,
                offset_y: 0,
            });
            for (x, y, text) in self.labels.iter() {
                draw_text_mut(
                    &mut imgbuf,
                    style.color,
                    (*x * width as f32) as i32 + style.offset_x,
                    (*y * height as f32) as i32 + style.offset_y,
                    style.scale,
                    &font,
                    text,
                );
            }
        }

        // Draw title
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
#[tokio::test]
async fn entry_point() {
    use chrono_tz::Asia::Tokyo;
    use common::binance::fetch_binance_kline_data;

    let candle_data = fetch_binance_kline_data::<Kline>("SOL_USDT", "1m", 60)
        .await
        .unwrap();
    let font_data = include_bytes!("../../Roboto-Light.ttf").to_vec();

    let png = Chart::new(Tokyo)
        .with_candle(candle_data)
        .with_title("SOL/USDT")
        .with_font_data(font_data)
        .with_candle_dimensions(10, 5)
        .with_points(vec![(0.5, 0.6), (0.25, 0.33), (0.8, 0.8)])
        .with_point_style(5, RGBColor(255, 0, 0)) // Red points with radius 5
        .with_lines(vec![[(0.5, 0.6), (0.25, 0.33)], [(0.8, 0.8), (0.5, 0.6)]])
        .with_line_style(3, RGBColor(0, 255, 0)) // Green lines with width 3
        .with_labels(vec![
            (0.5, 0.6, "A".to_string()),
            (0.25, 0.33, "B".to_string()),
            (0.8, 0.8, "C".to_string()),
        ])
        .with_label_style(20.0, 20.0, Rgb([0, 0, 255]), 10, 5) // Blue labels, larger scale, offset
        .build()
        .unwrap();

    std::fs::write("test.png", png).unwrap();
}
