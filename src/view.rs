#![allow(dead_code)]

use minifb::{Key, ScaleMode, Window, WindowOptions};
use num::Complex;
use ultraviolet::{DVec2, UVec2};

const OPAQUE: u32 = 0xFF_00_00_00;
const WHITE: u32 = 0xFF_FF_FF | OPAQUE;

#[allow(clippy::identity_op)]
const BLACK: u32 = 0x00_00_00 | OPAQUE;

fn rgb(r: u8, g: u8, b: u8) -> u32 {
    let (r, g, b) = (r as u32, g as u32, b as u32);

    (r) | (g << 8) | (b << 16) | OPAQUE
}

#[inline]
fn rand_color() -> u32 {
    use rand::RngCore;

    rand::thread_rng().next_u32() | OPAQUE
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SimConfig {
    /// 2D Dimensions of the framebuffer
    pixels: UVec2,

    /// Complex point of the lower-left (-x & -y) point of the frame
    frame_min: DVec2,

    /// Complex point of the upper-right (+x & +y) point of the frame
    frame_max: DVec2,
}

impl SimConfig {
    #[inline]
    fn idx_to_complex(&self, idx: u32) -> Complex<f64> {
        // Unpack out integer coordinates
        let x = idx % self.pixels.x;
        let y = idx / self.pixels.x;

        // Normalize coordinates
        let x: f64 = (x as f64) / (self.pixels.x as f64);
        let y: f64 = (y as f64) / (self.pixels.y as f64);

        // Flip the buffer to put "bigger" y at the "top"
        let y: f64 = 1.0 - y;

        // Scale into the bounds space
        let x = x * self.frame_max.x + (1.0 - x) * self.frame_min.x;
        let y = y * self.frame_max.y + (1.0 - y) * self.frame_min.y;

        Complex::new(x, y)
    }
}

#[derive(Copy, Clone, Debug)]
struct GridCell {
    c: Complex<f64>,
    z: Complex<f64>,
    iters: u32,
}

impl GridCell {
    fn new(c: Complex<f64>) -> Self {
        GridCell {
            c,
            z: Complex::new(0., 0.),
            iters: 0,
        }
    }
}

struct Sim {
    config: SimConfig,
    iters: u32,
    grid: Vec<GridCell>,
}

impl Sim {
    fn new(config: SimConfig) -> Self {
        let framebuffer_size = config.pixels.x * config.pixels.y;
        let mut grid = Vec::with_capacity(framebuffer_size as usize);

        for idx in 0..framebuffer_size {
            let c = config.idx_to_complex(idx);
            grid.push(GridCell::new(c));
        }

        assert_eq!(grid.len(), framebuffer_size as usize);

        Self {
            config,
            iters: 0,
            grid,
        }
    }

    fn update(&mut self) {
        self.iters += 1;

        for cell in self.grid.iter_mut() {
            // Skip already diverged cells
            if cell.z.norm_sqr() >= 4.0 {
                continue;
            }

            // Update the state of the grid
            cell.iters += 1;
            cell.z = cell.z * cell.z + cell.c;
        }
    }

    fn draw(&mut self, fb: &mut [u32]) {
        assert_eq!(fb.len(), self.grid.len());

        for (pixel, cell) in fb.iter_mut().zip(self.grid.iter()) {
            if cell.z.norm_sqr() <= 4.0 {
                *pixel = rgb(0, 0, 0);
                continue;
            }

            let ratio = 1.0 - (cell.iters as f64 / self.iters as f64);

            let g = (0xff as f64 * ratio) as u8;
            *pixel = rgb(g, g, g);
        }
    }
}

// Pick a reasonable resolution that fits without on screen and matches the frame's aspect ratio
fn pick_window_dims(min: DVec2, max: DVec2) -> UVec2 {
    // Approximate maximum resolution in each dimension that we want
    // I'm using the MacBook Air's maximum resolution scaled by 80%
    const SCALE: f64 = 0.8;
    let window_dims = SCALE * DVec2::new(1680., 1080.);

    // This is the ratio of the widdth of the window to the height
    // Greater than 1.0 is typical, and means the window is wider than it is tall.
    let window_ratio = window_dims.x / window_dims.y;
    let frame_ratio: f64 = {
        let dx = max.x - min.x;
        let dy = max.y - min.y;

        dx / dy
    };

    // We want to scale our dims so that they fit in the window, while still being as large as
    // we can get.
    let (x, y): (f64, f64);

    // We'll check the frame_ratio to tell which axis has to change.
    // One of three things will happen:
    use std::cmp::Ordering;
    match frame_ratio
        .partial_cmp(&window_ratio)
        .expect("Expected comparabile dimensions - no NaNs!")
    {
        Ordering::Equal => {
            // 1. The ratio happens to match the window's ratio, so we'll use it directly.
            x = window_dims.x;
            y = window_dims.y;
        }
        Ordering::Greater => {
            // 2. The frame is relatively wider than the window, so use the window's width and scale our height
            x = window_dims.x;
            y = window_dims.x / frame_ratio;
        }
        Ordering::Less => {
            // 3. The frame is relatively taller than the window, so use the window's height and scale our width
            x = window_dims.y * frame_ratio;
            y = window_dims.y;
        }
    }

    // Sanity check because this logic took forever to get right.
    assert!(x <= window_dims.x as f64);
    assert!(y <= window_dims.y as f64);

    // Round our chosen dimensions into integer coordinates and we're done!
    UVec2::new(x.round() as u32, y.round() as u32)
}

fn main() {
    // Visually appealing framing for the Mandelbrot set
    let frame_min: DVec2 = DVec2::new(-2.5, -1.25);
    let frame_max: DVec2 = DVec2::new(1.0, 1.25);

    let window_dims = pick_window_dims(frame_min, frame_max);

    let mut window = Window::new(
        &format!("Mandelbrot - {}x{}", window_dims.x, window_dims.y),
        window_dims.x as usize,
        window_dims.y as usize,
        WindowOptions {
            resize: true,
            scale_mode: ScaleMode::AspectRatioStretch,
            ..WindowOptions::default()
        },
    )
    .expect("Failed to create a window");

    let mut sim = Sim::new(SimConfig {
        pixels: window_dims,
        frame_min,
        frame_max,
    });

    let mut framebuffer: Vec<u32> = vec![0; (window_dims.x * window_dims.y) as usize];

    // Limit to max ~60 fps update rate
    window.limit_update_rate(Some(std::time::Duration::from_micros(16_600)));

    while window.is_open() {
        if window.is_key_down(Key::Escape) || window.is_key_down(Key::Q) {
            break;
        }

        sim.update();

        sim.draw(&mut framebuffer);

        if let Err(err) =
            window.update_with_buffer(&framebuffer, window_dims.x as usize, window_dims.y as usize)
        {
            dbg!(err);
        }
        window.update();
    }
}
