use godot::engine::{Image, Node};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct DclTestingTools {
    #[base]
    _base: Base<Node>,
}

#[godot_api]
impl DclTestingTools {
    #[func]
    fn compute_image_similarity(
        &self,
        mut img_a: Gd<Image>,
        mut img_b: Gd<Image>,
        save_diff_to_path: GString,
    ) -> f64 {
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

        let mut data_diff = Vec::with_capacity(a_data.len());
        for index in 0..a_data.len() {
            data_diff.push((data_a[index] as i32 - data_b[index] as i32) as i16);
        }

        if !save_diff_to_path.is_empty() {
            let mut diff_img = Image::create(
                width as i32,
                height as i32,
                false,
                godot::engine::image::Format::FORMAT_RGB8,
            )
            .expect("Failed to create diff image");

            let mut dest_data_packed_array = diff_img.get_data();
            let dest_data = dest_data_packed_array.as_mut_slice();
            for index in 0..a_data.len() {
                dest_data[index] = data_diff[index].unsigned_abs() as u8;
            }
            diff_img.set_data(
                width as i32,
                height as i32,
                false,
                godot::engine::image::Format::FORMAT_RGB8,
                dest_data_packed_array,
            );
            diff_img.save_png(save_diff_to_path);
        }

        let mut data_diff_factor = Vec::with_capacity(pixel_count);
        for pixel_index in 0..pixel_count {
            let index = pixel_index * 3;
            let [r, g, b] = &data_diff[index..index + 3] else {
                panic!("Invalid index");
            };
            let diff_sum_i = ((*r as i32) * (*r as i32))
                + ((*g as i32) * (*g as i32))
                + ((*b as i32) * (*b as i32));
            let diff_factor_i = (diff_sum_i as f64) / (3. * (u8::MAX as f64).powi(2));
            data_diff_factor.push(1.0 - diff_factor_i);
        }

        let score: f64 = (data_diff_factor.iter().sum::<f64>() / (pixel_count as f64)).sqrt();

        score
    }

    #[func]
    fn exit_gracefully(&self, code: i32) {
        std::process::exit(code);
    }
}
