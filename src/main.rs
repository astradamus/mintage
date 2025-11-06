mod material;
mod physics;
mod reaction;
mod sim;
mod world;

use std::sync::atomic::Ordering;
use macroquad::prelude::*;
use sim::{TpsTracker, spawn_sim_thread};

// Constants
const WORLD_TICKS_PER_SECOND: f64 = 20.0;

fn window_conf() -> Conf {
    Conf {
        window_title: "coinage 0.1.0".to_owned(),
        window_width: 2400,
        window_height: 1400,
        fullscreen: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {

    // World size multiplier, for convenient tile size calculations.
    let multi = 16.0;

    // World size in tiles.
    let w = (32.0*multi) as usize;
    let h = (16.0*multi) as usize;

    // Tile size in pixels.
    let tile_size: f32 = 64.0 / multi as f32;
    let world_px_w = (w as f32 * tile_size) as u32;
    let world_px_h = (h as f32 * tile_size) as u32;

    // Spawn Sim thread, hold on to shared state.
    let shared = spawn_sim_thread(w, h);

    // Tracks ticks per second.
    let mut tps_tracker = TpsTracker::new();

    // Render loop.
    let mut img = Image::gen_image_color(w as u16, h as u16, BLACK);
    let tex = Texture2D::from_image(&img);
    tex.set_filter(FilterMode::Nearest);

    loop {

        // Get latest snapshot from shared state.
        let snapshot = shared.current.load();

        // Draw world to render target.
        clear_background(Color::from_rgba(10, 12, 16, 255));
        for y in 0..snapshot.h {
            for x in 0..snapshot.w {
                if let Some(mat) = shared.mat_db.get(&snapshot.mat_id_at(x, y)) {
                    img.set_pixel(x as u32, y as u32, mat.color);
                }
            }
        }

        // Draw texture to screen
        tex.update(&img);
        set_default_camera();

        let sw = screen_width();
        let sh = screen_height();
        let scale_x = sw / world_px_w as f32;
        let scale_y = sh / world_px_h as f32;
        let scale = scale_x.min(scale_y).floor().max(1.0);

        let dest_w = world_px_w as f32 * scale;
        let dest_h = world_px_h as f32 * scale;
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
        let step = shared.tick_count.load(Ordering::Relaxed);
        let tps = tps_tracker.update(&shared);
        let total_time = get_time();

        draw_text(&format!("Sim Step: {}", step),                   10.0, 24.0*1.0, 24.0, BLUE);
        draw_text(&format!("TPS: {}", tps),                         10.0, 24.0*2.0, 24.0, SKYBLUE);
        draw_text(&format!("Real Secs: {}", total_time),            10.0, 24.0*3.0, 24.0, SKYBLUE);

        let wtps = WORLD_TICKS_PER_SECOND;
        draw_text(&format!("SPS: {}", tps / wtps),                  10.0, 24.0*4.0, 24.0, PURPLE);
        draw_text(&format!("World Secs: {}", step / wtps as u64),   10.0, 24.0*5.0, 24.0, PURPLE);

        next_frame().await;
    }
}
