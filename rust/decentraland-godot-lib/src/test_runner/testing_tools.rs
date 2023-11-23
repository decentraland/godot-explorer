use ethers::core::k256::elliptic_curve::consts::U9;
use godot::engine::{Image, Node};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct DclTestingTools {
    #[base]
    _base: Base<Node>,
}

fn to_gray(rgb: &[u8]) -> f32 {
    let r = rgb[0];
    let g = rgb[1];
    let b = rgb[2];

    ((0.299 * f32::from(r) + 0.587 * f32::from(g) + 0.114 * f32::from(b)) / u8::MAX as f32).round()
        as f32
}

#[godot_api]
impl DclTestingTools {
    #[func]
    fn compute_image_similarity(&self, mut img_a: Gd<Image>, mut img_b: Gd<Image>) -> f64 {
        if img_a.get_width() != img_b.get_width() || img_a.get_height() != img_b.get_height() {
            tracing::info!("compute_image_similarity have different sizes");
            return 0.0;
        }

        let width = img_a.get_width() as usize;
        let height = img_a.get_height() as usize;
        let pixel_count = width * height;

        if img_a.get_format() != godot::engine::image::Format::FORMAT_RGB8 {
            img_a.convert(godot::engine::image::Format::FORMAT_RGB8);
        }

        if img_b.get_format() != godot::engine::image::Format::FORMAT_RGB8 {
            img_b.convert(godot::engine::image::Format::FORMAT_RGB8);
        }

        let a_data = img_a.get_data();
        let b_data = img_b.get_data();
        let data_a = a_data.as_slice();
        let data_b = b_data.as_slice();
        let mut data_diff = Vec::with_capacity(pixel_count);

        for pixel_index in 0..pixel_count {
            let index = pixel_index * 3;
            data_diff.push(
                1.0 - (to_gray(&data_b[index..index + 3]) - to_gray(&data_a[index..index + 3]))
                    .abs(),
            );
        }
        let score: f64 = 1.
            - (data_diff
                .iter()
                .map(|p| (1. - *p as f64).powi(2))
                .sum::<f64>()
                / (pixel_count as f64))
                .sqrt();

        score
    }
}
