use anyhow::{anyhow, bail, ensure, Context, Result};
use egui::ColorImage;
use tracing::{debug, error, info, trace, warn};

use image::{ImageBuffer, Rgb, RgbImage, Rgba, RgbaImage};

use crate::{model::ObjectData, model_orca::OrcaModel};

pub fn model_to_image(size: egui::Vec2, model: &OrcaModel) -> Result<ColorImage> {
    let mult: f32 = 2.;
    // let (imgx, imgy) = (size.x as u32, size.y as u32);
    let (imgx, imgy) = (256 * mult as u32, 256 * mult as u32);

    let mut img_buf = RgbaImage::new(imgx, imgy);

    let (bed_x, bed_y) = (256., 256.);

    debug!("drawing meshes");
    for (pos, mesh) in model.get_meshes().iter() {
        for t in mesh.triangles.triangle.iter() {
            let v1 = &mesh.vertices.vertex[t.v1 as usize];
            let v2 = &mesh.vertices.vertex[t.v2 as usize];
            let v3 = &mesh.vertices.vertex[t.v3 as usize];

            for (a, b) in [(v1, v2), (v2, v3), (v3, v1)] {
                let a = [a.x + pos[0], a.y + pos[1]];
                let b = [b.x + pos[0], b.y + pos[1]];

                let start = ((a[0] as f32 * mult) as i32, (a[1] as f32 * mult) as i32);
                let end = ((b[0] as f32 * mult) as i32, (b[1] as f32 * mult) as i32);

                // let start = ((a.x as f32 * mult) as i32, (a.y as f32 * mult) as i32);
                // let end = ((b.x as f32 * mult) as i32, (b.y as f32 * mult) as i32);
                imageproc::drawing::draw_antialiased_line_segment_mut(
                    &mut img_buf,
                    start,
                    end,
                    Rgba([64, 64, 64, 255]),
                    imageproc::pixelops::interpolate,
                );
            }
        }
    }

    // for x in 15..=17 {
    //     for y in 8..24 {
    //         img_buf.put_pixel(x, y, Rgba([255, 0, 0, 255]));
    //         img_buf.put_pixel(y, x, Rgba([255, 0, 0, 255]));
    //     }
    // }

    img_buf.save("preview.png")?;

    debug!("resizing image");
    let img_buf = image::imageops::resize(
        &img_buf,
        size.x as u32,
        size.y as u32,
        image::imageops::FilterType::CatmullRom,
    );

    let pixels = img_buf.as_flat_samples();
    debug!("pixels.len() = {}", pixels.samples.len());
    let img = ColorImage::from_rgba_unmultiplied([size.x as _, size.y as _], pixels.as_slice());

    // unimplemented!()
    // img
    Ok(img)
}
