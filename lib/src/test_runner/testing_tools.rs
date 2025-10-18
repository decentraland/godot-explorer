use godot::engine::Image;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct DclTestingTools {
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

        if img_a.get_format() != godot::engine::image::Format::RGB8 {
            img_a.convert(godot::engine::image::Format::RGB8);
        }

        if img_b.get_format() != godot::engine::image::Format::RGB8 {
            img_b.convert(godot::engine::image::Format::RGB8);
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
                godot::engine::image::Format::RGB8,
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
                godot::engine::image::Format::RGB8,
                dest_data_packed_array,
            );
            diff_img.save_png(save_diff_to_path);
        }

        // ---------------- Similarity computation ----------------
        // Accumulate the squared channel differences directly to avoid extra
        // allocations and possible integer overflows.
        let mut summatory: u64 = 0;
        for pixel_index in 0..pixel_count {
            let index = pixel_index * 3;
            let [r, g, b] = &data_diff[index..index + 3] else {
                panic!("Invalid index");
            };
            let dr = *r as i64;
            let dg = *g as i64;
            let db = *b as i64;
            summatory += (dr * dr + dg * dg + db * db) as u64;
        }
        let summatory = summatory as f64;

        // Do the large multiplication in floating-point space to prevent any
        // intermediate usize overflows.
        let denom = 3.0 * (u8::MAX as f64).powi(2) * pixel_count as f64;
        let factor = 1.0 / denom;
        let score: f64 = (1.0 - factor * summatory).sqrt();

        score
    }

    #[func]
    fn exit_gracefully(&self, code: i32) {
        use godot::engine::{Engine, SceneTree};

        if let Some(main_loop) = Engine::singleton().get_main_loop() {
            let mut tree = main_loop.cast::<SceneTree>();
            tree.quit_ex().exit_code(code).done();
        } else {
            // Fallback to process exit if we can't get the main loop (shouldn't happen)
            tracing::warn!("Could not get SceneTree, falling back to std::process::exit");
            std::process::exit(code);
        }
    }
}
