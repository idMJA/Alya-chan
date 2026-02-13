use skia_safe::{
    surfaces, Canvas, ClipOp, Color, Data, EncodedImageFormat, Font, FontMgr, FontStyle, Image,
    Paint, Point, RRect, Rect, Typeface,
};

pub struct ShipCanvas {
    font: Option<Typeface>,
}

impl ShipCanvas {
    pub const fn new() -> Self {
        Self { font: None }
    }

    pub fn load_font(mut self, path: &str) -> Self {
        if let Ok(data) = std::fs::read(path) {
            let data = Data::new_copy(&data);
            if let Some(tf) = FontMgr::default().new_from_data(&data, None) {
                self.font = Some(tf);
                tracing::info!("Font loaded: {}", path);
            } else {
                tracing::warn!("Failed to load font from data: {}", path);
            }
        } else {
            tracing::warn!("Font file not found: {}", path);
        }
        self
    }

    pub fn load_bg_bytes() -> Option<Vec<u8>> {
        match std::fs::read("assets/utils/ship.png") {
            Ok(bytes) => {
                tracing::info!(
                    "Background image loaded successfully: {} bytes",
                    bytes.len()
                );
                Some(bytes)
            }
            Err(e) => {
                tracing::warn!("Failed to read background image file: {}", e);
                None
            }
        }
    }

    pub fn generate(
        &self,
        name1: &str,
        name2: &str,
        avatar1: Option<Vec<u8>>,
        avatar2: Option<Vec<u8>>,
        percentage: u32,
        bg_bytes: Option<&[u8]>,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut surface =
            surfaces::raster_n32_premul((1280, 720)).ok_or("Failed to create surface")?;
        let canvas = surface.canvas();

        self.draw_bg(canvas, bg_bytes);

        if let Some(data) = avatar1 {
            self.draw_avatar(canvas, &data, 619.4, 205.1, 97.2)?;
        }
        if let Some(data) = avatar2 {
            self.draw_avatar(canvas, &data, 151.0, 433.7, 97.2)?;
        }

        self.draw_text(canvas, name1, 415.0, 254.3);
        self.draw_text(canvas, name2, 464.7, 481.0);
        self.draw_percent(canvas, percentage, 440.6, 363.5);

        let img = surface.image_snapshot();

        // i have a skill issue, then i js fck it with deprecated method
        #[allow(deprecated)]
        let data = img
            .encode_to_data(EncodedImageFormat::PNG)
            .ok_or("Encode failed")?;
        Ok(data.as_bytes().to_vec())
    }

    fn draw_bg(&self, canvas: &Canvas, bg_bytes: Option<&[u8]>) {
        if let Some(bytes) = bg_bytes {
            tracing::debug!(
                "Attempting to decode background image: {} bytes",
                bytes.len()
            );
            if let Some(img) = Image::from_encoded(Data::new_copy(bytes)) {
                tracing::info!("Background image decoded successfully, drawing...");
                canvas.draw_image_rect(
                    img,
                    None,
                    Rect::from_xywh(0.0, 0.0, 1280.0, 720.0),
                    &Paint::default(),
                );
                return;
            }
            tracing::warn!("Failed to decode background image, using fallback gradient");
        } else {
            tracing::info!("No background bytes provided, using gradient fallback");
        }

        let mut paint = Paint::default();
        let colors = [
            Color::from_rgb(255, 107, 138),
            Color::from_rgb(217, 70, 239),
        ];
        let pos: &[f32] = &[0.0, 1.0];

        if let Some(shader) = skia_safe::Shader::linear_gradient(
            (Point::new(0.0, 0.0), Point::new(1280.0, 720.0)),
            skia_safe::gradient_shader::GradientShaderColors::Colors(&colors),
            Some(pos),
            skia_safe::TileMode::Clamp,
            None,
            None,
        ) {
            paint.set_shader(shader);
        }
        canvas.draw_rect(Rect::from_xywh(0.0, 0.0, 1280.0, 720.0), &paint);
    }

    fn draw_avatar(
        &self,
        canvas: &Canvas,
        data: &[u8],
        x: f32,
        y: f32,
        size: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let img = Image::from_encoded(Data::new_copy(data)).ok_or("Bad image")?;
        canvas.save();

        let rect = Rect::from_xywh(x, y, size, size);
        let radius = size / 2.0;
        let rounded = RRect::new_rect_xy(rect, radius, radius);

        canvas.clip_rrect(rounded, ClipOp::Intersect, true);

        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        canvas.draw_image_rect(img, None, rect, &paint);
        canvas.restore();
        Ok(())
    }

    fn draw_text(&self, canvas: &Canvas, name: &str, x: f32, y: f32) {
        let clean: String = name
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
            .to_uppercase();

        let mut paint = Paint::default();
        paint.set_color(Color::WHITE);
        paint.set_anti_alias(true);

        let tf = self.font.clone().unwrap_or_else(|| {
            let mgr = FontMgr::default();
            mgr.match_family_style("Arial", FontStyle::bold())
                .unwrap_or_else(|| mgr.legacy_make_typeface(None, FontStyle::bold()).unwrap())
        });

        let mut size = 40.0;
        let max_width = 373.7;
        let font = Font::from_typeface(tf.clone(), size);
        let (w, _) = font.measure_str(&clean, Some(&paint));

        if clean.len() > 10 && w > max_width {
            size = (40.0 * (max_width / w)).floor();
        }

        let final_font = Font::from_typeface(tf, size);
        let (tw, bounds) = final_font.measure_str(&clean, Some(&paint));
        let cx = x - (tw / 2.0);
        let text_height = bounds.height();
        let cy = y + (text_height / 2.0);

        canvas.draw_str(&clean, (cx, cy), &final_font, &paint);
    }

    fn draw_percent(&self, canvas: &Canvas, pct: u32, x: f32, y: f32) {
        let mut paint = Paint::default();
        paint.set_color(Color::WHITE);
        paint.set_anti_alias(true);

        let mgr = FontMgr::default();
        let tf = mgr
            .match_family_style("Arial", FontStyle::bold())
            .unwrap_or_else(|| mgr.legacy_make_typeface(None, FontStyle::bold()).unwrap());
        let font = Font::from_typeface(tf, 24.0);

        let text = format!("{pct}");
        let (_, bounds) = font.measure_str(&text, Some(&paint));
        let text_height = bounds.height();
        let cy = y + (text_height / 2.0);

        canvas.draw_str(&text, (x, cy), &font, &paint);
    }
}

pub fn calc_love(id1: &str, id2: &str) -> u32 {
    let combined = format!("{id1}{id2}");
    let hash: i32 = combined.chars().fold(0i32, |mut a, b| {
        a = ((a << 5).wrapping_sub(a)).wrapping_add(b as i32);
        a
    });
    u32::try_from(hash.abs() % 101).unwrap_or(0)
}

pub fn ship_name(n1: &str, n2: &str) -> String {
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    let mid1 = (n1.len() as f64 / 2.0).ceil() as usize;
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    let mid2 = (n2.len() as f64 / 2.0).floor() as usize;
    format!("{}{}", &n1[..mid1], &n2[mid2..])
}

pub const fn love_msg(pct: u32) -> (&'static str, &'static str) {
    match pct {
        90..=100 => ("💖", "Absolutely perfect! You two are soulmates! The universe conspired to bring you together! 💫"),
        80..=89 => ("💖", "Perfect match! You two are meant to be together! There's undeniable chemistry here! ✨"),
        70..=79 => ("💕", "Excellent compatibility! This relationship has all the right ingredients for success! 🌟"),
        60..=69 => ("💕", "Great compatibility! There's definitely something special brewing between you two! 💫"),
        50..=59 => ("💓", "Good potential! With some effort and understanding, this could blossom into something beautiful! 🌸"),
        40..=49 => ("💓", "Moderate compatibility. There are some sparks, but it might take work to fan the flames! 🔥"),
        30..=39 => ("💔", "Some chemistry detected, but there might be some challenges to overcome! 💪"),
        20..=29 => ("💔", "Limited compatibility. Friendship might be a better foundation than romance! 🤝"),
        10..=19 => ("💙", "Very little romantic chemistry. You're probably better as friends! 👫"),
        _ => ("💙", "No romantic spark detected! But hey, the best relationships often start as friendships! 💙"),
    }
}
