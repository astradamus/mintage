mod material;
mod physics;
mod reaction;
mod sim;
mod world;

use std::collections::HashMap;
use std::fs;
use std::sync::atomic::Ordering;
use macroquad::prelude::*;
use serde_json::Value;
use sim::{TpsTracker, spawn_sim_thread};

// Constants
const WORLD_TICKS_PER_SECOND: f64 = 20.0;
const COLORS_THERM_GRADIENT: [Color; 3] = [
    Color::from_rgba(0, 96, 255, 255),
    Color::from_rgba(115, 115, 115, 255),
    Color::from_rgba(255, 64, 0, 255),
];

fn window_conf() -> Conf {
    Conf {
        window_title: "mintage 0.1.0".to_owned(),
        window_width: 2400,
        window_height: 1400,
        fullscreen: false,
        ..Default::default()
    }
}

/// Generates a color from a ratio and a bundle of three colors (neg, zero, pos).
fn triple_gradient_bun(ratio: f32, bundle: &[Color]) -> Color {
    triple_gradient(ratio, bundle[0], bundle[1], bundle[2])
}

/// Generates a color from a ratio and gradient definition.
fn triple_gradient(ratio: f32, neg: Color, zero: Color, pos: Color) -> Color {
    let ratio = ratio.clamp(-1.0, 1.0);

    let (from, to, w) = if ratio < 0.0 {
        (zero, neg, -ratio)
    } else {
        (zero, pos, ratio)
    };

    let r = from.r + (to.r - from.r) * w;
    let g = from.g + (to.g - from.g) * w;
    let b = from.b + (to.b - from.b) * w;

    Color::new(r, g, b, 1.0)
}

#[macroquad::main(window_conf)]
async fn main() {

    // Load config from RON file.
    let contents = fs::read_to_string("assets/config.ron").expect("Missing config: config.ron");
    let config: HashMap<String, Value> = ron::de::from_str(&contents).unwrap();

    // World size in cells.
    let w = config.get("world_width").expect("Missing config: world_width")
        .as_u64().expect("Invalid config: world_width must be u64") as usize;
    let h = config.get("world_height").expect("Missing config: world_height")
        .as_u64().expect("Invalid config: world_height must be u64") as usize;

    // Thermal view temp range set in config.
    let thermal_view_range = config.get("thermal_view_range").expect("Missing config: thermal_view_range")
        .as_f64().expect("Invalid config: thermal_view_range must be f64") as f32;

    // Spawn Sim thread, hold on to shared state.
    let shared = spawn_sim_thread(config, w, h);

    // Tracks ticks per second.
    let mut tps_tracker = TpsTracker::new();

    // Render loop.
    let mut img = Image::gen_image_color(w as u16, h as u16, BLACK);
    let tex = Texture2D::from_image(&img);
    tex.set_filter(FilterMode::Nearest);

    let mut view_thermal = false;

    loop {
        // Toggle view mode.
        if is_key_pressed(KeyCode::Space) {
            view_thermal = !view_thermal;
        }

        // Get current tick count.
        let step = shared.tick_count.load(Ordering::Relaxed);

        // Get latest snapshot from shared state.
        let snapshot = shared.current.load();

        // Draw world to render target.
        clear_background(Color::from_rgba(10, 12, 16, 255));
        for y in 0..snapshot.h {
            for x in 0..snapshot.w {

                if let Some(mat) = shared.mat_db.get(snapshot.mat_id_at(x, y)) {
                    let mut mat_rgb = mat.color;

                    if view_thermal {
                        let t = ((snapshot.temp_at(x, y) - 50.0) / thermal_view_range).clamp(-1.0, 1.0);
                        let therm_rgb = triple_gradient_bun(t, &COLORS_THERM_GRADIENT);

                        let alpha = 0.75;
                        mat_rgb.r = mat_rgb.r + (therm_rgb.r - mat_rgb.r) * alpha;
                        mat_rgb.g = mat_rgb.g + (therm_rgb.g - mat_rgb.g) * alpha;
                        mat_rgb.b = mat_rgb.b + (therm_rgb.b - mat_rgb.b) * alpha;
                    }
                    img.set_pixel(x as u32, y as u32, mat_rgb);
                }
            }
        }

        // Draw texture to screen
        tex.update(&img);
        set_default_camera();

        let sw = screen_width();
        let sh = screen_height();
        let scale_x = sw / w as f32;
        let scale_y = sh / h as f32;
        let scale = scale_x.min(scale_y).floor().max(1.0);

        let dest_w = w as f32 * scale;
        let dest_h = h as f32 * scale;
        let dx = (sw - dest_w) * 0.5;
        let dy = (sh - dest_h) * 0.5;

        draw_texture_ex(
            &tex,
            dx,
            dy,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(dest_w, dest_h)),
                ..Default::default()
            },
        );

        // UI overlay
        let tps = tps_tracker.update(&shared);
        let total_time = get_time();

        // Mouse Tooltip
        let mouse_pos = mouse_position();
        if mouse_pos.0 >= dx && mouse_pos.0 < dx + dest_w && mouse_pos.1 >= dy && mouse_pos.1 < dy + dest_h {
            let grid_x = ((mouse_pos.0 - dx) / scale) as usize;
            let grid_y = ((mouse_pos.1 - dy) / scale) as usize;
            if grid_x < w && grid_y < h {
                let temp = snapshot.temp_at(grid_x, grid_y);
                draw_text(&format!("Temp: {:.1}°C", temp), sw - 200.0, 24.0*1.0, 24.0, WHITE);
                let mat_id = snapshot.mat_id_at(grid_x, grid_y);
                if let Some(mat) = shared.mat_db.get(mat_id) {
                    draw_text(&format!("Mat: {}", mat.name), sw - 200.0, 24.0*2.0, 24.0, WHITE);
                }
            }
        }

        // COL1
        draw_text(&format!("Sim Step: {}", step),                                                       10.0, 24.0*1.0, 24.0, BLUE);
        draw_text(&format!("TPS: {}", tps),                                                             10.0, 24.0*2.0, 24.0, SKYBLUE);
        draw_text(&format!("Real Secs: {}", total_time),                                                10.0, 24.0*3.0, 24.0, SKYBLUE);

        let wtps = WORLD_TICKS_PER_SECOND;
        draw_text(&format!("SPS: {}", tps / wtps),                                                      10.0, 24.0*4.0, 24.0, PURPLE);
        draw_text(&format!("World Secs: {}", step / wtps as u64),                                       10.0, 24.0*5.0, 24.0, PURPLE);
        draw_text(&format!("World Hours: {}", step as f32 / 60.0 / 60.0 / wtps as f32),                 10.0, 24.0*6.0, 24.0, PURPLE);

        draw_text(&format!("Press [SPACE] to toggle Thermal View."),                                    screen_width()/2.0 - 140.0, 12.0, 20.0, WHITE);

        // COL2
        // draw_text(&format!("Tiles: {} x {}  ({})", w, h, w*h),                                          500.0, 24.0*1.0, 24.0, PURPLE);
        // let meters_w = w as f32 / 2.0;
        // let meters_h = h as f32 / 2.0;
        // draw_text(&format!("Meters: {} x {}  ({})", meters_w, meters_h, meters_w * meters_h),           500.0, 24.0*2.0, 24.0, PURPLE);
        // let feet_w = meters_w * 3.28084;
        // let feet_h = meters_h * 3.28084;
        // draw_text(&format!("Feet: {} x {}  ({})", feet_w, feet_h, feet_w * feet_h),                     500.0, 24.0*3.0, 24.0, PURPLE);


        next_frame().await;
    }
}
