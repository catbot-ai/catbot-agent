use ab_glyph::{FontRef, PxScale};
use chrono::offset::Local;
use chrono::{DateTime, Duration, NaiveDate};
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

fn parse_time(t: &str) -> DateTime<Local> {
    let values: Vec<u32> = t.split("-").map(|s| s.parse().unwrap()).collect();
    NaiveDate::from_ymd_opt(values[0] as i32, values[1], values[2])
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_local_timezone(Local)
        .unwrap()
}

pub(crate) fn draw_candle(
    font_data: Vec<u8>,
    metadata: ChartMetaData,
    candle_data: Vec<(&'static str, f32, f32, f32, f32)>,
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
            parse_time(candle_data[0].0) + Duration::days(1),
            parse_time(candle_data[candle_data.len() - 1].0) - Duration::days(1),
        );

        let mut chart =
            ChartBuilder::on(&root).build_cartesian_2d(from_date..to_date, 110f32..135f32)?;

        chart
            .configure_mesh()
            .light_line_style(RGBColor(48, 48, 48))
            .draw()?;

        chart.draw_series(candle_data.iter().map(|x| {
            let color = if x.4 >= x.1 { GREEN } else { RED };
            CandleStick::new(
                parse_time(x.0),
                x.1,
                x.2,
                x.3,
                x.4,
                ShapeStyle::from(&color).filled(),
                ShapeStyle::from(&color).filled(),
                15,
            )
        }))?;
    }

    // Create image buffer
    let mut imgbuf: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(WIDTH, HEIGHT);
    imgbuf.copy_from_slice(buffer.as_slice());

    // Colors
    let white = Rgb([255u8, 255u8, 255u8]);

    // Draw points and lines
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
    let candle_data = vec![
        ("2019-04-25", 130.06, 131.37, 128.83, 129.15),
        ("2019-04-24", 125.79, 125.85, 124.52, 125.01),
    ];
    let font_data = include_bytes!("../../Roboto-Light.ttf").to_vec();
    let png = draw_candle(
        font_data,
        ChartMetaData {
            title: "Hello World!".to_string(),
        },
        candle_data,
    )
    .unwrap();
    std::fs::write("test.png", png).unwrap();
}
