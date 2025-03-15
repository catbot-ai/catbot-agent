use super::helpers::parse_kline_time;
use super::painters::*;
use crate::charts::png::encode_png;
use ab_glyph::FontArc;
use ab_glyph::PxScale;
use chrono_tz::Tz;
use common::Kline;
use common::LongShortSignal;
use common::OrderBook;
use image::{ImageBuffer, Rgb};
use plotters::prelude::*;
use std::collections::HashMap;
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

    pub fn with_past_candle(mut self, past_candle_data: Vec<Kline>) -> Self {
        self.past_candle_data = Some(past_candle_data);
        self
    }

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

        // Combine past_candle_data and predicted_candle into all_candle_data
        let mut all_candle_data = self.past_candle_data.clone().unwrap();
        let last_candle =self.past_candle_data.iter().last().expect("No data").last().expect("No data");
        let last_past_time =  if let Some(predicted_candles) = self.predicted_candle.clone() {
            all_candle_data.extend(predicted_candles);

            all_candle_data
            .last()
            .map(|kline| kline.open_time)
            .ok_or("No past candle data available")?
        } else {
            last_candle.close_time
        };
        let current_price = last_candle.close_price.parse::<f64>().expect("No data");

        let past_data = self.past_candle_data.as_deref().unwrap_or(&[]);

        let total_candles = all_candle_data.len();
        let total_width = total_candles as u32 * 10;
        let root_width = 1024;
        let root_height = 1024;

        let chart_width = 768;
        let margin_right = 1024 - chart_width;
        let right_offset_x = chart_width;

        let plot_width = total_width.max(root_width);
        let bar: (u32, u32) = (plot_width, root_height);
        let mut buffer = vec![0; (plot_width * root_height * 3) as usize];

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

        let mut lower_bound = 0.0;
        let mut upper_bound = 0.0;
        // Scope for drawing operations
        {
            let mut root = BitMapBackend::with_buffer(&mut buffer, bar).into_drawing_area();

             let (  new_lower_bound, new_upper_bound) = draw_chart(
                &mut root,
                &all_candle_data,
                past_data,
                timezone,
                &self,
                min_price,
                max_price,
                first_time,
                last_time,
                margin_right,
                plot_width,
                last_past_time,
                &self.timeframe,
            )?;
            lower_bound = new_lower_bound;
            upper_bound = new_upper_bound;

            let mut top_chart = ChartBuilder::on(&root.split_vertically((50).percent()).0)
                .margin_right(margin_right)
                .build_cartesian_2d(first_time..last_time, min_price * 0.95..max_price * 1.05)?;

         if let Some(ref past_signals) = self.past_signals  {
            draw_past_signals(&mut top_chart, timezone, past_signals)?;
         }
        } // `root` goes out of scope here, ending the borrow of `buffer`
 
        // Create imgbuf after root is dropped
        let mut imgbuf: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(plot_width, root_height);
        imgbuf.copy_from_slice(buffer.as_slice());

        let crop_x = plot_width.saturating_sub(root_width);
        let mut cropped_img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            image::imageops::crop_imm(&imgbuf, crop_x, 0, root_width, root_height).to_image();

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

            let section_height = root_height as f32 * 0.5 / num_indicators;
            let top_section_height = root_height as f32 * 0.5;

            let mut current_y = top_section_height  ;

            if self.volume_enabled {
                draw_volume_detail(&mut cropped_img, past_data, &font, current_y   )?;
                current_y += section_height ;
            }
            if self.macd_enabled {
                draw_macd_detail(&mut cropped_img, past_data, &font, current_y  )?;
                current_y += section_height ;
            }
            if self.stoch_rsi_enabled {
                draw_stoch_rsi_detail(&mut cropped_img, past_data, &font, current_y   )?;
            }
        }

        let price_bounding_rect = draw_axis_labels(
            &mut cropped_img,
            &font.clone(),
            past_data,
            &self,
            root_height,
            root_width,
            margin_right,
            min_price,
            max_price,
        )?;

        let mut bids_asks_y_map = HashMap::new();

        // Draw order book
        if let Some(orderbook_data) = &self.orderbook_data {
            if let Some(price_bounding_rect) = price_bounding_rect{
            let new_bids_asks_y_map = draw_order_book(
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

            bids_asks_y_map = new_bids_asks_y_map;
 }
        }

        // Draw signals if has
        if let Some(ref signals) = &self.signals {

            if let Some(price_bounding_rect) = price_bounding_rect{
            draw_signals( &mut cropped_img,
                &font,
                signals,
                current_price,
                price_bounding_rect,
            )?;}
        }

        // Draw labels if has
        draw_labels(&mut cropped_img, &font, &self, root_width, root_height)?;

        // Draw lines if has
        draw_lines(&mut cropped_img, &self, root_width, root_height)?;

        Ok(encode_png(&cropped_img)?)
    }
}

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

    let limit = 24 * 10; // 240 candles, enough for iteration
    let candle_data = fetch_binance_kline_data::<Kline>(binance_pair_symbol, timeframe, limit)
        .await
        .unwrap();

    let orderbook = fetch_orderbook_depth(binance_pair_symbol, 1000)
        .await
        .unwrap();

    // Past signals (unchanged from your code)
    let mut past_signals = Vec::new();
    if candle_data.len() >= 31 {
        // Long signal: entry at last - 10, target at last
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

        // Short signal: entry at last - 30, target at last - 20
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

    // Generate mock signals based on last candle
    let mut signals = Vec::new();
    if !candle_data.is_empty() {
        let last_candle = &candle_data[candle_data.len() - 1];
        let last_close_price = last_candle.close_price.parse::<f64>().unwrap();
        let last_time = last_candle.open_time;
        let hour_ms = 3_600_000; // 1 hour in milliseconds

        // Mock Long Signal: unchanged from original
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

        // Modified Mock Short Signal: 
        // Entry at current_price - 1%, targeting 20% profit downward
        let short_entry_time = long_target_time;
        let short_entry_price = last_close_price * 0.99; // Current price - 1%
        let short_target_price = short_entry_price * 0.80; // 20% profit (price decreases)
        let short_target_time = short_entry_time + hour_ms;

        signals.push(LongShortSignal {
            direction: "short".to_string(),
            symbol: binance_pair_symbol.to_string(),
            confidence: 0.87,
            current_price: short_entry_price,
            entry_price: short_entry_price,
            target_price: short_target_price,
            stop_loss: short_entry_price * 1.03, // 3% above entry
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

        // Debug prints for mock signals
        for signal in &signals {
            println!(
                "{} Signal: Entry Time: {}, Entry Price: {:.2}, Target Time: {}, Target Price: {:.2}, Stop Loss: {:.2}",
                signal.direction, signal.entry_time_local, signal.entry_price, 
                signal.target_time_local, signal.target_price, signal.stop_loss
            );
        }
    }

    // Get the mock prediction
    let predicted_klines_string = get_mock_graph_prediction().await;
    let predicted_klines = serde_json::from_str::<RefinedGraphPredictionResponse>(
        &predicted_klines_string.clone(),
    )
    .unwrap()
    .klines;

    // println!("{predicted_klines:#?}");

    let png = Chart::new(timeframe, Tokyo)
        .with_past_candle(candle_data)
        // .with_predicted_candle(predicted_klines) // Commented out as per original
        .with_title(binance_pair_symbol)
        .with_font_data(font_data)
        .with_volume()
        .with_macd()
        .with_stoch_rsi()
        .with_orderbook(orderbook)
        .with_bollinger_band()
        // .with_past_signals(past_signals.clone()) 
        .with_signals(signals)
        .build()
        .unwrap();

    std::fs::write("test.png", png).unwrap();
}
}
