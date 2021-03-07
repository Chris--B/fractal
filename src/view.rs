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

struct Sim {
    px_dim_x: u32,
    px_dim_y: u32,

    bounds_min_x: f64,
    bounds_min_y: f64,
    bounds_max_x: f64,
    bounds_max_y: f64,
}

impl Sim {
    fn new(config: SimConfig) -> Self {
        // TODO: Just save the config directly
        Self {
            px_dim_x: config.pixels.x,
            px_dim_y: config.pixels.y,

            bounds_min_x: config.frame_min.x,
            bounds_min_y: config.frame_min.y,
            bounds_max_x: config.frame_max.x,
            bounds_max_y: config.frame_max.y,
        }
    }

    #[inline]
    fn idx_to_complex(&self, idx: u32) -> Complex<f64> {
        // Unpack out integer coordinates
        let x = idx % self.px_dim_x;
        let y = idx / self.px_dim_x;

        // Normalize coordinates
        let x: f64 = (x as f64) / (self.px_dim_x as f64);
        let y: f64 = (y as f64) / (self.px_dim_y as f64);

        // Flip the buffer to put "bigger" y at the "top"
        let y: f64 = 1.0 - y;

        // Scale into the bounds space
        let x = x * self.bounds_max_x + (1.0 - x) * self.bounds_min_x;
        let y = y * self.bounds_max_y + (1.0 - y) * self.bounds_min_y;

        Complex::new(x, y)
    }

    fn draw(&mut self, fb: &mut [u32]) {
        /// Iterations that make it this far are assumed to never diverge and get colored black.
        const MAX_ITERS: u32 = 30;

        for (idx, pixel) in fb.iter_mut().enumerate() {
            let c = self.idx_to_complex(idx as u32);
            let mut z = Complex::new(0., 0.);

            let mut iters = MAX_ITERS;

            for i in 0..MAX_ITERS {
                z = z * z + c;
                if z.norm_sqr() >= 4.0 {
                    iters = i;
                }
            }

            let ratio = 1.0 - (iters as f64 / MAX_ITERS as f64);
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

    // Render once
    // This keeps the window interactive once the image has been rendered.
    sim.draw(&mut framebuffer);

    // Limit to max ~60 fps update rate
    window.limit_update_rate(Some(std::time::Duration::from_micros(16_600)));

    while window.is_open() {
        if window.is_key_down(Key::Escape) || window.is_key_down(Key::Q) {
            break;
        }

        if let Err(err) =
            window.update_with_buffer(&framebuffer, window_dims.x as usize, window_dims.y as usize)
        {
            dbg!(err);
        }
        window.update();
    }
}
