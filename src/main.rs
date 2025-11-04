mod material;
mod world;
mod physics;
mod reaction;

use macroquad::prelude::*;
use macroquad::rand::srand;
use world::{World};
use physics::{PhysicsEngine};
use crate::physics::{BasicReactions, SteamBehavior};

fn window_conf() -> Conf {
    Conf {
        window_title: "coinage 0.1.0".to_owned(),
        window_width: 1600,
        window_height: 800,
        fullscreen: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // Set seed.
    srand(12345689);

    let multi = 16.0;

    let w = (32.0*multi) as usize;
    let h = (16.0*multi) as usize;

    let mut world = World::new(w, h);
    let mut phys_eng = PhysicsEngine::new();

    // Basic random map
    {
        let (curr, mut next) = world.ctx_pair();

        for y in 0..h {
            for x in 0..w {
                let result = rand::gen_range(0.0, 1.0);
                if result < 0.01 {
                    next.set_mat_id(x, y, curr.materials.get_id("base:blood").unwrap());
                }
                else if result < 0.2 {
                    next.set_mat_id(x, y, curr.materials.get_id("base:water").unwrap());
                }
                else if result < 0.25 {
                    next.set_mat_id(x, y, curr.materials.get_id("base:lava").unwrap());
                }
                else {
                    next.set_mat_id(x, y, curr.materials.get_id("base:air").unwrap());
                }
            }
        }
        world.swap_all();

    }
    // Physics modules
    {
        let (curr, mut next) = world.ctx_pair();
        // Reactions must go first, or changes made by other modules will prevent reactions in changed cells.
        // TODO Swap between modules.
        phys_eng.add(BasicReactions::new(&curr));
        phys_eng.add(SteamBehavior::new(&curr));
    }


    let tile_size: f32 = 64.0 / multi as f32;
    let world_px_w = (w as f32 * tile_size) as u32;
    let world_px_h = (h as f32 * tile_size) as u32;

    let zoom = vec2(2.0 / world_px_w as f32, 2.0 / world_px_h as f32);
    let target = vec2(world_px_w as f32 / 2.0, world_px_h as f32 / 2.0);

    // Main loop
    let mut step_timer: u64 = 0;
    let mut autoplay = true;

    // Map draw
    let mut img = Image::gen_image_color(w as u16, h as u16, BLACK);
    let tex = Texture2D::from_image(&img);
    tex.set_filter(FilterMode::Nearest);

    loop {

        // Input
        if is_key_pressed(KeyCode::Space) {
            step_timer += 1;
            phys_eng.step(&mut world);
        }
        if is_key_down(KeyCode::Space) && is_key_down(KeyCode::LeftShift) {
            step_timer += 1;
            phys_eng.step(&mut world);
        }

        if autoplay {
            step_timer += 1;
            phys_eng.step(&mut world);
        }

        // Draw world to render target
        clear_background(Color::from_rgba(10, 12, 16, 255));
        for y in 0..world.h {
            for x in 0..world.w {
                if let Some(mat) = world.get_curr_mat_at(x, y) {
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
        draw_text("Space to Step", 10.0, 24.0, 24.0, WHITE);
        draw_text(&format!("Sim Step: {step_timer}"), 10.0, 48.0, 24.0, WHITE);


        let fps = get_fps();
        let total_time = get_time(); // seconds since app start (f64)
        draw_text(&format!("Seconds: {}", total_time), 10.0, 24.0*3.0, 24.0, WHITE);
        draw_text(&format!("FPS: {}", step_timer as f64/total_time), 10.0, 24.0*4.0, 24.0, WHITE);
        draw_text(&format!("FPS: {}", fps), 10.0, 24.0*5.0, 24.0, WHITE);

        next_frame().await;
    }
}
