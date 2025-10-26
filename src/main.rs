mod material;
mod world;
mod physics;
mod reaction;

use macroquad::prelude::*;
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
    let multi = 1;

    let w = 32*multi;
    let h = 16*multi;

    let mut world = World::new(w, h);
    let mut phys_eng = PhysicsEngine::new();

    // Basic random map
    {
        let (read, mut write) = world.ctx_pair();

        for y in 0..h {
            for x in 0..w {
                let result = rand::gen_range(0.0, 1.0);
                if result < 0.01 {
                    write.cell_mut(x, y).mat_id = read.materials.get_id("base:blood").unwrap();
                }
                else if result < 0.2 {
                    write.cell_mut(x, y).mat_id = read.materials.get_id("base:water").unwrap();
                }
                else if result < 0.25 {
                    write.cell_mut(x, y).mat_id = read.materials.get_id("base:lava").unwrap();
                }
                else {
                    write.cell_mut(x, y).mat_id = read.materials.get_id("base:air").unwrap();
                }
            }
        }
        world.swap_all();

    }
    // Physics modules
    {
        let (read, mut write) = world.ctx_pair();
        // Reactions must go first, or changes made by other modules will prevent reactions in changed cells.
        phys_eng.add(BasicReactions::new(&read));
        phys_eng.add(SteamBehavior::new(&read));
    }


    let tile_size: f32 = 32.0 / multi as f32;
    let world_px_w = (w as f32 * tile_size) as u32;
    let world_px_h = (h as f32 * tile_size) as u32;

    let zoom = vec2(2.0 / world_px_w as f32, 2.0 / world_px_h as f32);
    let target = vec2(world_px_w as f32 / 2.0, world_px_h as f32 / 2.0);

    let rt = render_target(world_px_w, world_px_h);
    rt.texture.set_filter(FilterMode::Nearest);

    // Main loop
    let mut step_timer: u64 = 0;
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

        // Camera
        let cam_world = Camera2D {
            render_target: Some(rt.clone()),
            zoom,
            target,
            ..Default::default()
        };
        set_camera(&cam_world);

        // Draw world to render target
        clear_background(Color::from_rgba(10, 12, 16, 255));
        for y in 0..world.h {
            for x in 0..world.w {
                if let Some(mat) = world.mat_at(x, y) {
                    let rx = x as f32 * tile_size;
                    let ry = y as f32 * tile_size;
                    draw_rectangle(rx, ry, tile_size -1.0, tile_size -1.0, mat.color);
                }
            }
        }

        // Draw render target to window
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
            &rt.texture,
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

        next_frame().await;
    }
}
