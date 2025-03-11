use super::helpers::parse_kline_time;
use super::painters::*;
use crate::charts::png::encode_png;
use ab_glyph::FontArc;
use ab_glyph::PxScale;
use chrono_tz::Tz;
use common::Kline;
use image::{ImageBuffer, Rgb};
use plotters::coord::types::RangedCoordf32;
use plotters::prelude::*;
use std::error::Error;

// Styling structures
#[derive(Default, Clone)]
pub struct PointStyle {
    pub radius: i32,
    pub color: RGBColor,
}

#[derive(Default, Clone)]
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

#[derive(Default, Clone)]
pub struct ChartMetaData {
    pub title: String,
}

// Chart struct
#[derive(Default, Clone)]
pub struct Chart {
    pub timezone: Tz,
    pub timeframe: String,
    pub past_candle_data: Option<Vec<Kline>>,
    pub metadata: ChartMetaData,
    pub font_data: Option<Vec<u8>>,
    pub candle_width: u32,
    pub points: Vec<(f32, f32)>,
    pub point_style: Option<PointStyle>,
    pub lines: Vec<[(f32, f32); 2]>,
    pub line_style: Option<LineStyle>,
    pub labels: Vec<(f32, f32, String)>,
    pub label_style: Option<LabelStyle>,
    pub macd_enabled: bool,
    pub bollinger_enabled: bool,
    pub volume_enabled: bool,
    pub stoch_rsi_enabled: bool,
    pub long_signals: Vec<(i64, i64, f32, f32)>, // (entry_time, target_time, entry_price, target_price)
    pub short_signals: Vec<(i64, i64, f32, f32)>, // (entry_time, target_time, entry_price, target_price)
}

impl Chart {
    pub fn new(timeframe: &str, timezone: Tz) -> Self {
        Chart {
            timeframe: timeframe.to_string(),
            timezone,
            ..Default::default()
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

    pub fn with_long_signals(mut self, signals: Vec<(i64, i64, f32, f32)>) -> Self {
        self.long_signals = signals;
        self
    }

    pub fn with_short_signals(mut self, signals: Vec<(i64, i64, f32, f32)>) -> Self {
        self.short_signals = signals;
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

        // Scope for drawing operations
        {
            let mut root = BitMapBackend::with_buffer(&mut buffer, bar).into_drawing_area();
            draw_chart(
                &mut root,
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

            let mut top_chart = ChartBuilder::on(&root.split_vertically((50).percent()).0)
                .margin_right(margin_right)
                .build_cartesian_2d(first_time..last_time, min_price * 0.95..max_price * 1.05)?;

            draw_point_on_candle(
                &mut top_chart,
                timezone,
                &self.long_signals,
                &self.short_signals,
            )?;
        } // `root` goes out of scope here, ending the borrow of `buffer`

        // Create imgbuf after root is dropped
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

        let limit = 24 * 10; // 240 candles, enough for iteration
        let candle_data = fetch_binance_kline_data::<Kline>(pair_symbol, timeframe, limit)
            .await
            .unwrap();

        // Generate mock signals based on fixed candle ranges
        let mut long_signals = Vec::new();
        let mut short_signals = Vec::new();

        if candle_data.len() >= 31 {
            // Long signal: entry at last - 10, target at last
            let last_candle = &candle_data[candle_data.len() - 1];
            let last_minus_10_candle = &candle_data[candle_data.len() - 11]; // last - 10

            let long_entry_time = last_minus_10_candle.open_time;
            let long_entry_price = last_minus_10_candle.close_price.parse::<f32>().unwrap();
            let long_target_time = last_candle.open_time;
            let long_target_price = last_candle.close_price.parse::<f32>().unwrap();

            long_signals.push((
                long_entry_time,
                long_target_time,
                long_entry_price,
                long_target_price,
            ));

            // Short signal: entry at last - 30, target at last - 20
            let last_minus_30_candle = &candle_data[candle_data.len() - 31]; // last - 30
            let last_minus_20_candle = &candle_data[candle_data.len() - 21]; // last - 20

            let short_entry_time = last_minus_30_candle.open_time;
            let short_entry_price = last_minus_30_candle.close_price.parse::<f32>().unwrap();
            let short_target_time = last_minus_20_candle.open_time;
            let short_target_price = last_minus_20_candle.close_price.parse::<f32>().unwrap();

            short_signals.push((
                short_entry_time,
                short_target_time,
                short_entry_price,
                short_target_price,
            ));

            // Debug prints to verify
            println!(
                "Long Signal: Entry Time: {}, Entry Price: {}, Target Time: {}, Target Price: {}",
                long_entry_time, long_entry_price, long_target_time, long_target_price
            );
            println!(
                "Short Signal: Entry Time: {}, Entry Price: {}, Target Time: {}, Target Price: {}",
                short_entry_time, short_entry_price, short_target_time, short_target_price
            );
        } else {
            println!(
                "Not enough candles to generate mock signals. Need at least 31 candles, got {}",
                candle_data.len()
            );
        }

        let png = Chart::new(timeframe, Tokyo)
            .with_candle_width(6)
            .with_past_candle(candle_data)
            .with_title(pair_symbol)
            .with_font_data(font_data)
            .with_volume()
            .with_macd()
            .with_stoch_rsi()
            .with_bollinger_band()
            .with_long_signals(long_signals)
            .with_short_signals(short_signals)
            .build()
            .unwrap();

        std::fs::write("test.png", png).unwrap();
    }
}
