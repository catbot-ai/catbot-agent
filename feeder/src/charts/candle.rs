use ab_glyph::{FontRef, PxScale};
use chrono::{DateTime, Duration};
use chrono_tz::Tz;
use common::Kline;
use image::ImageEncoder;
use image::Rgb;
use image::{codecs::png::PngEncoder, ImageBuffer, ImageError, Pixel};
use imageproc::drawing::draw_text_mut;
use plotters::coord::types::RangedCoordf32;
use plotters::prelude::*;
use std::ops::Deref;

pub struct ChartMetaData {
    pub title: String,
}

// Convert Kline timestamp (i64) to DateTime<Tz>
fn parse_kline_time(timestamp: i64, tz: &Tz) -> DateTime<Tz> {
    DateTime::from_timestamp(timestamp / 1000, 0)
        .unwrap()
        .with_timezone(tz)
}

pub fn draw_candle(
    font_data: Vec<u8>,
    metadata: ChartMetaData,
    candle_data: Vec<Kline>,
    timezone: &Tz,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let font = FontRef::try_from_slice(&font_data)?;

    // Raw
    const WIDTH: u32 = 1024;
    const HEIGHT: u32 = 768;
    const BAR: (u32, u32) = (WIDTH, HEIGHT);
    const BAZ: usize = (WIDTH * HEIGHT * 3) as usize;
    let mut buffer = vec![0; BAZ];

    // Draw chart
    {
        let root = BitMapBackend::with_buffer(&mut buffer, BAR).into_drawing_area();
        root.fill(&BLACK)?;

        let (to_date, from_date) = (
            parse_kline_time(candle_data[0].open_time, timezone) + Duration::days(1),
            parse_kline_time(candle_data[candle_data.len() - 1].open_time, timezone)
                - Duration::days(1),
        );

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

        let mut chart = ChartBuilder::on(&root).build_cartesian_2d(
            from_date..to_date,
            min_price * 0.95..max_price * 1.05, // Add some padding to the Y-axis
        )?;

        chart
            .configure_mesh()
            .light_line_style(RGBColor(48, 48, 48))
            .draw()?;

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
                10,
            )
        }))?;
    }

    // Create image buffer
    let mut imgbuf: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(WIDTH, HEIGHT);
    imgbuf.copy_from_slice(buffer.as_slice());

    // Colors
    let white = Rgb([255u8, 255u8, 255u8]);

    // Draw points and lines (example, adjust as needed)
    {
        let root = BitMapBackend::with_buffer(&mut imgbuf, BAR).into_drawing_area();
        let root = root.apply_coord_spec(Cartesian2d::<RangedCoordf32, RangedCoordf32>::new(
            0f32..1f32,
            0f32..1f32,
            (0..WIDTH as i32, 0..HEIGHT as i32),
        ));

        let points = [(0.5, 0.6), (0.25, 0.33), (0.8, 0.8)];

        for (x, y) in points.iter() {
            root.draw(&Circle::new((*x, *y), 3, ShapeStyle::from(&WHITE).filled()))?;
        }

        for window in points.windows(2) {
            if let [p1, p2] = window {
                root.draw(&PathElement::new(
                    vec![(*p1), (*p2)],
                    ShapeStyle::from(&YELLOW).stroke_width(2),
                ))?;
            }
        }
    }

    // Font scales
    let title_scale = PxScale { x: 50.0, y: 50.0 };
    let point_scale = PxScale { x: 15.0, y: 15.0 };

    // Draw all text in a separate scope
    {
        let points = [(0.5, 0.6), (0.25, 0.33), (0.8, 0.8)];
        for (x, y) in points.iter() {
            draw_text_mut(
                &mut imgbuf,
                white,
                (x * WIDTH as f32) as i32 + 5,
                (y * HEIGHT as f32) as i32,
                point_scale,
                &font,
                &format!("({:.2},{:.2})", x, y),
            );
        }

        // Title
        draw_text_mut(
            &mut imgbuf,
            white,
            400,
            10,
            title_scale,
            &font,
            &metadata.title,
        );
    }

    Ok(encode_png(&imgbuf)?)
}

fn encode_png<P, Container>(img: &ImageBuffer<P, Container>) -> Result<Vec<u8>, ImageError>
where
    P: Pixel<Subpixel = u8> + image::PixelWithColorType + 'static,
    Container: Deref<Target = [P::Subpixel]>,
{
    let mut buf = Vec::new();
    let encoder = PngEncoder::new(&mut buf);
    encoder.write_image(img, img.width(), img.height(), P::COLOR_TYPE)?;
    Ok(buf)
}

#[test]
fn entry_point() {
    use chrono_tz::Asia::Tokyo;

    let candle_data = vec![
        Kline {
            open_time: 1556150400000, // 2019-04-25 00:00:00 UTC in milliseconds
            open_price: "130.06".to_string(),
            high_price: "131.37".to_string(),
            low_price: "128.83".to_string(),
            close_price: "129.15".to_string(),
            volume: "1000".to_string(),
            close_time: 1556236799999,
            quote_asset_volume: "10000".to_string(),
            number_of_trades: 500,
            taker_buy_base_asset_volume: "500".to_string(),
            taker_buy_quote_asset_volume: "5000".to_string(),
            ignore: "0".to_string(),
        },
        Kline {
            open_time: 1556064000000, // 2019-04-24 00:00:00 UTC in milliseconds
            open_price: "125.79".to_string(),
            high_price: "125.85".to_string(),
            low_price: "124.52".to_string(),
            close_price: "125.01".to_string(),
            volume: "800".to_string(),
            close_time: 1556150399999,
            quote_asset_volume: "8000".to_string(),
            number_of_trades: 400,
            taker_buy_base_asset_volume: "400".to_string(),
            taker_buy_quote_asset_volume: "4000".to_string(),
            ignore: "0".to_string(),
        },
    ];
    let font_data = include_bytes!("../../Roboto-Light.ttf").to_vec();
    let png = draw_candle(
        font_data,
        ChartMetaData {
            title: "Hello World!".to_string(),
        },
        candle_data,
        &Tokyo,
    )
    .unwrap();
    std::fs::write("test.png", png).unwrap();
}
