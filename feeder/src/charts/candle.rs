use super::helpers::get_visible_range_and_data;
use super::helpers::parse_kline_time;
use super::image::draw_dashed_line_segment_mut;
use super::painters::*;
use crate::charts::png::encode_png;
use ab_glyph::FontArc;
use ab_glyph::PxScale;
use chrono::DateTime;
use chrono::Utc;
use chrono_tz::Tz;
use common::Kline;
use common::LongShortSignal;
use common::OrderBook;
use image::Rgba;
use image::{ImageBuffer, Rgb};
use imageproc::drawing::draw_line_segment_mut;
use imageproc::drawing::text_size;
use plotters::prelude::*;
use std::error::Error;
use image::buffer::ConvertBuffer;

// Styling structures (unchanged)
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
    pub color: Rgba<u8>,
    pub background_color: Rgba<u8>,
    pub offset_x: i32,
    pub offset_y: i32,
}

#[derive(Default, Clone)]
pub struct ChartMetaData {
    pub title: String,
}

// Chart struct (unchanged)
#[derive(Default, Clone)]
pub struct Chart {
    pub timezone: Tz,
    pub timeframe: String,
    pub past_candle_data: Option<Vec<Kline>>,
    pub predicted_candle: Option<Vec<Kline>>,
    pub metadata: ChartMetaData,
    pub font_data: Option<Vec<u8>>,
    pub points: Vec<(f32, f32)>,
    pub orderbook_data: Option<OrderBook>,
    pub point_style: Option<PointStyle>,
    pub lines: Vec<[(f32, f32); 2]>,
    pub line_style: Option<LineStyle>,
    pub labels: Vec<(f32, f32, String)>,
    pub label_style: Option<LabelStyle>,
    pub macd_enabled: bool,
    pub bollinger_enabled: bool,
    pub volume_enabled: bool,
    pub stoch_rsi_enabled: bool,
    pub signals: Option<Vec<LongShortSignal>>,
    pub past_signals: Option<Vec<LongShortSignal>>,
}

impl Chart {
    pub fn new(timeframe: &str, timezone: Tz) -> Self {
        Chart {
            timeframe: timeframe.to_string(),
            timezone,
            ..Default::default()
        }
    }

    // Other methods (with_*) remain unchanged
    pub fn with_past_candle(mut self, past_candle_data: Vec<Kline>) -> Self {
        self.past_candle_data = Some(past_candle_data);
        self
    }

    #[allow(dead_code)]
    pub fn with_predicted_candle(mut self, predicted_candle: Vec<Kline>) -> Self {
        self.predicted_candle = Some(predicted_candle);
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
    pub fn with_orderbook(mut self, orderbook_data: OrderBook) -> Self {
        self.orderbook_data = Some(orderbook_data);
        self
    }

    #[allow(dead_code)]
    pub fn with_label_style(
        mut self,
        scale_x: f32,
        scale_y: f32,
        color: Rgba<u8>,
        background_color: Rgba<u8>,
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

    #[allow(dead_code)]
    pub fn with_past_signals(mut self, past_signals: Vec<LongShortSignal>) -> Self {
        self.past_signals = Some(past_signals);
        self
    }

    #[allow(dead_code)]
    pub fn with_signals(mut self, signals: Vec<LongShortSignal>) -> Self {
        self.signals = Some(signals);
        self
    }

    #[allow(clippy::type_complexity)]
    fn get_visible_time_range(
        &self,
        all_candles: &[Kline],
        timezone: &Tz,
        candle_width: u32,
        chart_width: u32,
    ) -> Result<(DateTime<Tz>, DateTime<Tz>, Vec<Kline>), Box<dyn Error>> {
        let (start_visible, end_visible, visible_candles) = get_visible_range_and_data(
            all_candles,
            timezone,
            candle_width,
            chart_width,
        )?;

        // Ensure start_visible is earlier than end_visible
        if start_visible > end_visible {
            return Ok((end_visible, start_visible, visible_candles));
        }

        Ok((start_visible, end_visible, visible_candles))
    }

    #[allow(clippy::too_many_arguments, unused)]
    fn draw_low_high_labels(
        &self,
        img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
        font: &FontArc,
        visible_candles: &[Kline],
        start_visible: DateTime<Tz>,
        end_visible: DateTime<Tz>,
        min_price: f32,
        max_price: f32,
        chart_width: u32,
        chart_height: f64,
        candle_width: f32, // Add candle_width parameter
    ) -> Result<(), Box<dyn Error>> {
        // Find lowest and highest prices in the visible range
        let mut lowest_price = f32::INFINITY;
        let mut highest_price = f32::NEG_INFINITY;
        let mut lowest_price_time: i64 = 0;
        let mut highest_price_time: i64 = 0;

        for kline in visible_candles.iter() {
            let low = kline.low_price.parse::<f32>().unwrap();
            let high = kline.high_price.parse::<f32>().unwrap();

            if low < lowest_price {
                lowest_price = low;
                lowest_price_time = kline.open_time;
            }
            if high > highest_price {
                highest_price = high;
                highest_price_time = kline.open_time;
            }
        }

        // Map timestamps to x-coordinates using visible range
        let visible_time_range = end_visible.timestamp() as f64 - start_visible.timestamp() as f64;
        let chart_width_f32 = chart_width as f64;

        let lowest_x = if visible_time_range != 0.0 {
            ((parse_kline_time(lowest_price_time, &self.timezone).timestamp() as f64 - start_visible.timestamp() as f64) / visible_time_range * chart_width_f32) as f32
        } else {
            0.0
        };
        let highest_x = if visible_time_range != 0.0 {
            ((parse_kline_time(highest_price_time, &self.timezone).timestamp() as f64 - start_visible.timestamp() as f64) / visible_time_range * chart_width_f32) as f32
        } else {
            0.0
        };

        let candle_w2 = candle_width / 2.0;
        let chart_width2 = chart_width as f32 / 2.0;

        // Map prices to y-coordinates
        let price_range = (max_price * 1.05 - min_price * 0.95) as f64;
        let lowest_y = if price_range != 0.0 {
            (chart_height * (1.0 - ((lowest_price - min_price * 0.95) as f64 / price_range))) as f32
        } else {
            chart_height as f32 / 2.0
        };
        let highest_y = if price_range != 0.0 {
            (chart_height * (1.0 - ((highest_price - min_price * 0.95) as f64 / price_range))) as f32
        } else {
            chart_height as f32 / 2.0
        };

        // Calculate label top-left coordinates
        let label_low_x = lowest_x + candle_w2;
        let label_low_y = lowest_y + 8.0; 

        let label_high_y = highest_y - 20.0 - 8.0;
        let label_high_x = highest_x;

        // Adjust lowest_x for the center of the candlestick
        let lowest_x_center = lowest_x + candle_w2;
        let highest_x_center = highest_x;

        // Draw hallow labels
        let label_width = 112.0;
        let label_scale = PxScale { x: 20.0, y: 20.0 };
        let font_color = Rgba([255, 255, 255, 255]);
        let border_color = Rgba([255, 255, 255, 255]);

        let label_low_x = if label_low_x > chart_width2   { lowest_x - label_width - candle_w2 } else { lowest_x + 16.0 };
        let low_bounding_rect = draw_hallow_label(
            img,
            font,
            &format!("LOW:{:.2}", lowest_price),
            label_low_x,
            label_low_y,
            label_scale,
            font_color,
            border_color,
        )?;

        let label_high_x = if label_high_x > chart_width2   { highest_x - label_width - candle_w2 } else { highest_x  + 16.0 };

        // Under other label?
        let label_high_x = if label_high_x < 300.0 { 300.0 } else { label_high_x };

        let high_bounding_rect = draw_hallow_label(
            img,
            font,
            &format!("HIGH:{:.2}", highest_price),
            label_high_x,
            label_high_y,
            label_scale,
            font_color,
            border_color,
        )?;

        // Draw line from candlestick to the LOW label
        let line_color = Rgba([255, 255, 255, 255]); // White line
        
        let line_x2 = if label_low_x > chart_width2 { low_bounding_rect.left() + low_bounding_rect.width() as i32} else {label_low_x as i32};
        draw_line_segment_mut(
            img,
            (lowest_x_center, lowest_y),  
            (line_x2 as f32, lowest_y + 8.0),  
            line_color,
        );

        let line_x2 = if label_high_x > chart_width2 { high_bounding_rect.left() + high_bounding_rect.width() as i32} else {label_high_x as i32};
        draw_line_segment_mut(
            img,
            (highest_x_center, highest_y),
            (line_x2 as f32, highest_y - 8.0),  
            line_color,
        );

        // Horizon line
        let line_color = Rgba([255, 255, 255, 255/2u8]); // White line
        draw_dashed_line_segment_mut(
            img,
            (0.0, lowest_y),  
            (chart_width as f32, lowest_y),  
            3.0,
            3.0,
            line_color,
        );

        draw_dashed_line_segment_mut(
            img,
            (0.0, highest_y),  
            (chart_width as f32, highest_y),  
            3.0,
            3.0,
            line_color,
        );

        Ok(())
    }

    pub fn build(self) -> Result<Vec<u8>, Box<dyn Error>> {
        if self.past_candle_data.is_none() {
            return Err("Candle data set is required".into());
        }

        let font_data = self
            .font_data
            .as_ref()
            .ok_or("Font data is required")?
            .clone();
        let font = FontArc::try_from_vec(font_data)?;
        let timezone = &self.timezone;

        let mut all_candles = self.past_candle_data.clone().unwrap();
        let last_candle = all_candles.last().expect("No data").clone();
        let last_past_time = if let Some(predicted_candles) = self.predicted_candle.clone() {
            all_candles.extend(predicted_candles);
            all_candles
                .last()
                .map(|kline| kline.open_time)
                .ok_or("No past candle data available")?
        } else {
            last_candle.close_time
        };
        let current_price = last_candle.close_price.parse::<f64>().expect("No data");

        let past_candles = self.past_candle_data.as_deref().unwrap_or(&[]);

        let total_candles = all_candles.len();
        let total_width = total_candles as u32 * 10;
        let root_width = 1024;
        let root_height = 1024;
        let chart_width = 768;
        let margin_right = root_width - chart_width;
        let right_offset_x = chart_width;
        let plot_width = total_width.max(root_width);
        let bar: (u32, u32) = (plot_width, root_height);

        let mut buffer = vec![0; (plot_width * root_height * 3) as usize];

        let first_candle_time = parse_kline_time(all_candles[0].open_time, timezone);
        let last_candle_time = parse_kline_time(all_candles[all_candles.len() - 1].open_time, timezone);

        let prices: Vec<f32> = all_candles
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

        #[allow(unused_assignments)]
        let (lower_bound, upper_bound) = 
        {
            let mut root_area = BitMapBackend::with_buffer(&mut buffer, bar).into_drawing_area();
            self.draw_candles(
                &all_candles,
                past_candles,
                timezone,
                min_price,
                max_price,
                first_candle_time,
                last_candle_time,
                margin_right,
                plot_width,
                last_past_time,
                &mut root_area,
            )?
        };

        let mut imgbuf: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(plot_width, root_height);
        imgbuf.copy_from_slice(buffer.as_slice());

        let crop_x = plot_width.saturating_sub(root_width);
        let cropped_img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            image::imageops::crop_imm(&imgbuf, crop_x, 0, root_width, root_height).to_image();

        let mut cropped_img: ImageBuffer<Rgba<u8>, Vec<u8>> = cropped_img.convert();

        let candle_width = 9.0; // Same as used in get_visible_time_range
        let (start_visible, end_visible, visible_candles) = self.get_visible_time_range(
            &all_candles,
            timezone,
            candle_width as u32,
            chart_width,
        )?;

        // Pass candle_width to draw_low_high_labels
        self.draw_low_high_labels(
            &mut cropped_img,
            &font,
            &visible_candles,
            start_visible,
            end_visible,
            min_price,
            max_price,
            chart_width,
            (root_height as f32 * 0.5) as f64,
            candle_width, // Pass candle_width
        )?;

        let label_scale = PxScale { x: 20.0, y: 20.0 };
        let label_color = Rgba([255, 255, 255, 255]);
        let background_color = Rgba([0, 0, 0, 255]);
        let chart_bottom_y = (root_height as f32 * 0.5) - 20.0;

        let start_label = start_visible.format("%Y-%m-%d %H:%M").to_string();
        draw_label(
            &mut cropped_img,
            &font,
            &start_label,
            8.0,
            chart_bottom_y,
            label_scale,
            label_color,
            Some(background_color),
        )?;

        // let end_label = end_visible.format("%Y-%m-%d %H:%M").to_string();
        let now = Utc::now();
        let end_label = now.with_timezone(&chrono_tz::Asia::Tokyo).format("%Y-%m-%d %H:%M").to_string();
        let (end_label_width, _) = text_size(label_scale, &font, &end_label);
        draw_label(
            &mut cropped_img,
            &font,
            &end_label,
            (chart_width - end_label_width - 8) as f32,
            chart_bottom_y,
            label_scale,
            label_color,
            Some(background_color),
        )?;

        draw_candle_detail(&mut cropped_img, &self, &font)?;
        if self.bollinger_enabled {
            draw_bollinger_detail(&mut cropped_img, past_candles, &font)?;
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

            let section_height = root_height as f32 * 0.5 / num_indicators;
            let top_section_height = root_height as f32 * 0.5;

            let mut current_y = top_section_height;

            if self.volume_enabled {
                draw_volume_detail(&mut cropped_img, past_candles, &font, current_y)?;
                current_y += section_height;
            }
            if self.macd_enabled {
                draw_macd_detail(&mut cropped_img, past_candles, &font, current_y)?;
                current_y += section_height;
            }
            if self.stoch_rsi_enabled {
                draw_stoch_rsi_detail(&mut cropped_img, past_candles, &font, current_y)?;
            }
        }

        let price_bounding_rect = draw_axis_labels(
            &mut cropped_img,
            &font.clone(),
            past_candles,
            &self,
            root_height,
            root_width,
            margin_right,
            min_price,
            max_price,
        )?;

        if let Some(orderbook_data) = &self.orderbook_data {
            if let Some(price_bounding_rect) = price_bounding_rect {
                draw_orderbook(
                    &mut cropped_img,
                    &font,
                    orderbook_data,
                    min_price,
                    max_price,
                    root_width,
                    root_height,
                    right_offset_x as f32,
                    price_bounding_rect.top() as f32,
                    lower_bound,
                    upper_bound,
                    price_bounding_rect,
                )?;
            }
        }

        if let Some(ref signals) = &self.signals {
            if let Some(price_bounding_rect) = price_bounding_rect {
                draw_signals(
                    &mut cropped_img,
                    &font,
                    signals,
                    current_price,
                    price_bounding_rect,
                )?;
            }
        }

        draw_labels(&mut cropped_img, &font, &self, root_width, root_height)?;
        draw_lines(&mut cropped_img, &self, root_width, root_height)?;

        Ok(encode_png(&cropped_img)?)
    }

    #[allow(clippy::too_many_arguments, unused)]
    fn draw_candles(
        &self,
        all_candles: &[Kline],
        past_candles: &[Kline],
        timezone: &Tz,
        min_price: f32,
        max_price: f32,
        first_candle_time: DateTime<Tz>,
        last_candle_time: DateTime<Tz>,
        margin_right: u32,
        plot_width: u32,
        last_past_time: i64,
        root_area: &mut DrawingArea<BitMapBackend, plotters::coord::Shift>,
    ) -> Result<(f32, f32), Box<dyn Error>> {
        let mut top_chart = ChartBuilder::on(&root_area.split_vertically((50).percent()).0)
            .margin_right(margin_right)
            .build_cartesian_2d(first_candle_time..last_candle_time, min_price * 0.95..max_price * 1.05)?;

        let (lower_bound, upper_bound) = draw_chart(
            root_area,
            all_candles,
            past_candles,
            timezone,
            self,
            min_price,
            max_price,
            first_candle_time,
            last_candle_time,
            margin_right,
            plot_width,
            last_past_time,
            &self.timeframe,
        )?;

        if let Some(ref past_signals) = self.past_signals {
            draw_past_signals(&mut top_chart, timezone, past_signals)?;
        }

        Ok((lower_bound, upper_bound))
    }
 }

// Test module (unchanged)
#[cfg(test)]
mod test {
    use super::*;
    use chrono_tz::Asia::Tokyo;
    use common::binance::fetch_binance_kline_data;
    use common::binance::fetch_orderbook_depth;
    use common::cooker::get_mock_graph_prediction;
    use common::RefinedGraphPredictionResponse;

    #[tokio::test]
    async fn entry_point() {
        let binance_pair_symbol = "SOLUSDT";
        let timeframe = "1h";
        let font_data = include_bytes!("../../RobotoMono-Regular.ttf").to_vec();

        let limit = 24 * 10;
        let candle_data = fetch_binance_kline_data::<Kline>(binance_pair_symbol, timeframe, limit)
            .await
            .unwrap();

        let orderbook = fetch_orderbook_depth(binance_pair_symbol, 1000)
            .await
            .unwrap();

        let mut past_signals = Vec::new();
        if candle_data.len() >= 31 {
            let last_candle = &candle_data[candle_data.len() - 1];
            let last_minus_10_candle = &candle_data[candle_data.len() - 11];

            let long_entry_time = last_minus_10_candle.open_time;
            let long_entry_price = last_minus_10_candle.close_price.parse::<f64>().unwrap();
            let long_target_time = last_candle.open_time;
            let long_target_price = last_candle.close_price.parse::<f64>().unwrap();

            past_signals.push(LongShortSignal {
                direction: "long".to_string(),
                symbol: binance_pair_symbol.to_string(),
                confidence: 0.85,
                current_price: long_target_price,
                entry_price: long_entry_price,
                target_price: long_target_price,
                stop_loss: long_entry_price * 0.95,
                timeframe: timeframe.to_string(),
                entry_time: long_entry_time,
                target_time: long_target_time,
                entry_time_local: chrono::DateTime::<chrono::Utc>::from_timestamp(long_entry_time / 1000, 0)
                    .unwrap()
                    .with_timezone(&chrono_tz::Asia::Tokyo)
                    .to_string(),
                target_time_local: chrono::DateTime::<chrono::Utc>::from_timestamp(long_target_time / 1000, 0)
                    .unwrap()
                    .with_timezone(&chrono_tz::Asia::Tokyo)
                    .to_string(),
                rationale: "Mock long signal based on price movement".to_string(),
            });

            let last_minus_30_candle = &candle_data[candle_data.len() - 31];
            let last_minus_20_candle = &candle_data[candle_data.len() - 21];

            let short_entry_time = last_minus_30_candle.open_time;
            let short_entry_price = last_minus_30_candle.close_price.parse::<f64>().unwrap();
            let short_target_time = last_minus_20_candle.open_time;
            let short_target_price = last_minus_20_candle.close_price.parse::<f64>().unwrap();

            past_signals.push(LongShortSignal {
                direction: "short".to_string(),
                symbol: binance_pair_symbol.to_string(),
                confidence: 0.82,
                current_price: short_target_price,
                entry_price: short_entry_price,
                target_price: short_target_price,
                stop_loss: short_entry_price * 1.05,
                timeframe: timeframe.to_string(),
                entry_time: short_entry_time,
                target_time: short_target_time,
                entry_time_local: chrono::DateTime::<chrono::Utc>::from_timestamp(short_entry_time / 1000, 0)
                    .unwrap()
                    .with_timezone(&chrono_tz::Asia::Tokyo)
                    .to_string(),
                target_time_local: chrono::DateTime::<chrono::Utc>::from_timestamp(short_target_time / 1000, 0)
                    .unwrap()
                    .with_timezone(&chrono_tz::Asia::Tokyo)
                    .to_string(),
                rationale: "Mock short signal based on price movement".to_string(),
            });

            for signal in &past_signals {
                println!(
                    "{} Signal: Entry Time: {}, Entry Price: {}, Target Time: {}, Target Price: {}, Stop Loss: {}",
                    signal.direction, signal.entry_time_local, signal.entry_price, 
                    signal.target_time_local, signal.target_price, signal.stop_loss
                );
            }
        } else {
            println!(
                "Not enough candles to generate mock signals. Need at least 31 candles, got {}",
                candle_data.len()
            );
        }

        let mut signals = Vec::new();
        if !candle_data.is_empty() {
            let last_candle = &candle_data[candle_data.len() - 1];
            let last_close_price = last_candle.close_price.parse::<f64>().unwrap();
            let last_time = last_candle.open_time;
            let hour_ms = 3_600_000;

            let long_entry_time = last_time + hour_ms;
            let long_entry_price = last_close_price - 1.0;
            let long_target_price = long_entry_price * 1.10;
            let long_target_time = long_entry_time + hour_ms;

            signals.push(LongShortSignal {
                direction: "long".to_string(),
                symbol: binance_pair_symbol.to_string(),
                confidence: 0.9,
                current_price: long_entry_price,
                entry_price: long_entry_price,
                target_price: long_target_price,
                stop_loss: long_entry_price * 0.97,
                timeframe: timeframe.to_string(),
                entry_time: long_entry_time,
                target_time: long_target_time,
                entry_time_local: chrono::DateTime::<chrono::Utc>::from_timestamp(long_entry_time / 1000, 0)
                    .unwrap()
                    .with_timezone(&chrono_tz::Asia::Tokyo)
                    .to_string(),
                target_time_local: chrono::DateTime::<chrono::Utc>::from_timestamp(long_target_time / 1000, 0)
                    .unwrap()
                    .with_timezone(&chrono_tz::Asia::Tokyo)
                    .to_string(),
                rationale: "Mock long signal expecting 5% upward movement".to_string(),
            });

            let short_entry_time = long_target_time;
            let short_entry_price = last_close_price * 0.99;
            let short_target_price = short_entry_price * 0.80;
            let short_target_time = short_entry_time + hour_ms;

            signals.push(LongShortSignal {
                direction: "short".to_string(),
                symbol: binance_pair_symbol.to_string(),
                confidence: 0.87,
                current_price: short_entry_price,
                entry_price: short_entry_price,
                target_price: short_target_price,
                stop_loss: short_entry_price * 1.03,
                timeframe: timeframe.to_string(),
                entry_time: short_entry_time,
                target_time: short_target_time,
                entry_time_local: chrono::DateTime::<chrono::Utc>::from_timestamp(short_entry_time / 1000, 0)
                    .unwrap()
                    .with_timezone(&chrono_tz::Asia::Tokyo)
                    .to_string(),
                target_time_local: chrono::DateTime::<chrono::Utc>::from_timestamp(short_target_time / 1000, 0)
                    .unwrap()
                    .with_timezone(&chrono_tz::Asia::Tokyo)
                    .to_string(),
                rationale: "Mock short signal targeting 20% profit from 1% below current price".to_string(),
            });

            for signal in &signals {
                println!(
                    "{} Signal: Entry Time: {}, Entry Price: {:.2}, Target Time: {}, Target Price: {:.2}, Stop Loss: {:.2}",
                    signal.direction, signal.entry_time_local, signal.entry_price, 
                    signal.target_time_local, signal.target_price, signal.stop_loss
                );
            }
        }

        let predicted_klines_string = get_mock_graph_prediction().await;
        let predicted_klines = serde_json::from_str::<RefinedGraphPredictionResponse>(
            &predicted_klines_string.clone(),
        )
        .unwrap()
        .klines;

        let png = Chart::new(timeframe, Tokyo)
            .with_past_candle(candle_data)
            .with_title(binance_pair_symbol)
            .with_font_data(font_data)
            .with_volume()
            .with_macd()
            .with_stoch_rsi()
            .with_orderbook(orderbook)
            .with_bollinger_band()
            .with_signals(signals)
            .build()
            .unwrap();

        std::fs::write("test.png", png).unwrap();
    }
}