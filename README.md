# Mintage
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/built_with-Rust-dca282.svg)

**Mintage** is a high-performance, multithreaded 2D cellular physics engine built in Rust. Inspired by "Falling Sand" games and *Noita*, it is designed to serve as the underlying simulation layer for a traditional roguelike, providing a reactive and emergent world where temperature, materials, and chemistry interact dynamically. This repository currently includes the engine and a demo/visualizer. Roguelike gameplay systems are on the roadmap.

![Mintage demo](demo.gif)
> In this gif, orange is lava, blue is water, green is plant, teal is ice. Thermal view is toggled every 3 seconds. Ice melts to water, water evaporates to steam. Steam moves about and condenses to water when it loses enough heat. Plants grow when water is nearby, and burn to ash when temperature is too high.

## Features
- **Parallel Modularity**: Physics behaviors are implemented as discrete modules (Thermal Diffusion, Transformations, Reactions, etc.) that run all costly calculations in parallel using `rayon`.
- **Intent-Based Resolution**: Modules register changes as lightweight `Intents`. A single, fast resolver pass applies intents sequentially using priorities and rules to handle conflicts deterministically.
- **Double-Buffered World State**: World state is stored in double-buffers (read vs. write), ensuring thread and memory safety.
- **Dynamic Thermal System**: Heat conduction and thermal phase changes.
- **Data-Driven Material Engine**: Materials and reactions are defined in external `.ron` files for rapid iteration without recompiling.
- **Seed-Deterministic RNG**: All randomness is seeded deterministically, ensuring reproducible results across runs.
- **High-Performance Visualization**: Rendering runs on its own thread, reading world state from an `ArcSwap` and displaying it using the `macroquad` game engine.
    -   **Material View**: Standard pixel-grid rendering.
    -   **Thermal View**: Real-time gradient heat map overlay.
    -   **Live Inspection**: Hover tooltips for precise cell data (Material & Temperature).

## Architecture
The engine is designed around predictable memory access, deterministic resolution, and parallel-friendly simulation steps. 
- The world is composed of a grid of cells stored in a cache-friendly **Structure of Arrays** format.
- For each cell, the following data is stored:
    - u16: ID referencing a material definition (color, diffusivity, etc.).
    - f32: The current temperature of the cell.

### Intents (how modules cooperate)
Modules do not mutate the world state directly. Instead, they emit intents, like:
- "Add X to this cell's temperature."
- "Change this cell's material to Y."
- "Swap the contents (material/temperature) of two cells."

Each tick, after all modules have run in parallel, the resolver writes all intents to the world state, resolving conflicts deterministically.

## Controls

| Key     | Action                                             |
|:--------|:---------------------------------------------------|
| `Space` | Toggle thermal overlay                             |
| `Mouse` | Hover over any cell to see detailed info in the UI |

## Getting Started

### Prerequisites
- Rust + Cargo (stable)

### Running the Demo
```shell script
# Clone the repository
git clone https://github.com/astradamus/mintage.git
cd mintage

# Run in release mode for maximum simulation speed
cargo run --release
```


## Configuration

### Engine (`assets/config.ron`)
Define simulation parameters.
```ron
{
    "steam_fade_chance": 0.0,       // For demo purposes it's more interesting if steam never fades.
    "world_width": 580,             // World size in cells.
    "world_height": 300,            // World size in cells.

    // Controls how much temperature variation fits into the thermal view color gradient.
    // Larger values show more detail at extreme temperatures but compress differences near zero.
    // Values beyond +/- range appear fully red or fully blue.
    "thermal_view_range": 500.0
}
```

### Initial World State (`assets/map.png` and `assets/map_key.ron`)
Using hex color codes (case-insensitive), define colors on the bitmap and their corresponding material and starting temperature. Multiple colors can correspond to the same material at different temperatures.
```ron
{
    "#000000": (    // black
        material: "base:air",
        temperature: 50.0,
    ),
    "#0000ff": (    // blue
        material: "base:water",
        temperature: 50.0,
    ),
    "#00ffff": (    // teal
        material: "base:ice",
        temperature: -1000.0,
    ),
    "#ff0000": (    // red
        material: "base:lava",
        temperature: 100000.0,
    ),
    "#00ff00": (    // green
        material: "base:plant",
        temperature: 50.0,
    ),
}
```

### Materials (`assets/materials_base.ron`)
Define the physical properties and visual representation of world elements.
```ron
{
    "base:water": (
        color_raw: (40, 120, 255, 255),
        diffusivity: 0.01,    // 0.0 for perfect insulation, 0.25 for perfect conduction
        transform_cold_mat_name: "base:ice",
        transform_cold_temp: 0.0,
        transform_hot_mat_name: "base:steam",
        transform_hot_temp: 100.0,
    ),
}
```

### Reactions (`assets/reactions_base.ron`)
Define how elements interact.
```ron
{
    "base:plant+water=plant+plant": (
        in_a: "base:plant",
        in_b: "base:water",
        out_a: "base:plant",
        out_b: "base:plant",
        rate: 0.005,    // Percent chance of reaction occurring per tick.
    ),
}
```

## Roadmap
- [ ] Exothermic/endothermic reactions.
- [ ] Fire and explosions.
- [ ] Novel physics mechanics. (Magic physics?)
- [ ] Entities (player, monsters, etc.).
- [ ] Saving/Loading world states.

## License
This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.