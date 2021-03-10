#![allow(dead_code)]

use minifb::{Key, ScaleMode, Window, WindowOptions};
use num::Complex;
use ultraviolet::{DVec2, UVec2};

use std::time::{Duration, Instant};

const WHITE: u32 = rgb(0xff, 0xff, 0xff);
const BLACK: u32 = rgb(0x00, 0x00, 0x00);
const OPAQUE: u32 = 0xFF_00_00_00;

/// Construct a color for use with minifb
///
/// The encoding for each pixel is 0RGB
const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    let (r, g, b) = (r as u32, g as u32, b as u32);

    (r << 16) | (g << 8) | b | OPAQUE
}

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

/// Maps iteration cycles to a color
///
/// Sourced from StackOverflow: https://stackoverflow.com/a/16505538
const COLOR_MAPPING: [u32; 16] = [
    rgb(66, 30, 15),
    rgb(25, 7, 26),
    rgb(9, 1, 47),
    rgb(4, 4, 73),
    rgb(0, 7, 100),
    rgb(12, 44, 138),
    rgb(24, 82, 177),
    rgb(57, 125, 209),
    rgb(134, 181, 229),
    rgb(211, 236, 248),
    rgb(241, 233, 191),
    rgb(248, 201, 95),
    rgb(255, 170, 0),
    rgb(204, 128, 0),
    rgb(153, 87, 0),
    rgb(106, 52, 3),
];

fn color_by_iteration(i: u32) -> u32 {
    COLOR_MAPPING[i as usize % COLOR_MAPPING.len()]
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

    /// Reset the sim state to a fresh object
    fn reset(&mut self) {
        self.grid.clear();

        let framebuffer_size = self.config.pixels.x * self.config.pixels.y;
        for idx in 0..framebuffer_size {
            let c = self.config.idx_to_complex(idx);
            self.grid.push(GridCell::new(c));
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

            if cell.iters == self.iters {
                // If we haven't escaped yet, use black
                *pixel = BLACK;
            } else {
                // otherwise, we'll use a palette that cycles between neat colors
                *pixel = color_by_iteration(cell.iters);
            }
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

/// Make a square frame centered at `p` with radius `r`
fn make_square_frame(p: DVec2, r: f64) -> (DVec2, DVec2) {
    let min: DVec2 = DVec2::new(p.x - r, p.y - r);
    let max: DVec2 = DVec2::new(p.x + r, p.y + r);

    (min, max)
}

/// Makes a frame that shows "The" Mandelbrot fractal how everyone expects it
fn make_default_frame() -> (DVec2, DVec2) {
    let min: DVec2 = DVec2::new(-2.5, -1.25);
    let max: DVec2 = DVec2::new(1.0, 1.25);

    (min, max)
}

#[allow(unused_variables)]
fn main() {
    // Frames taken from here:
    // http://www.cuug.ab.ca/dewara/mandelbrot/Mandelbrowser.html

    // X = -1.25066
    // Y = 0.02012
    // R = 1.7E-4
    let (frame_min, frame_max) = make_square_frame((-1.25066, 0.02012).into(), 1.7E-4);

    // X = -0.722
    // Y = 0.246
    // R = 0.019
    let (frame_min, frame_max) = make_square_frame((-0.722, 0.246).into(), 0.019);

    // "The" Mandelbrot view
    let (frame_min, frame_max) = make_default_frame();

    // dimensions for the window
    let window_dims = pick_window_dims(frame_min, frame_max);

    // dimsensions for the framebuffer
    // scale this up for better quality
    let pixel_dims = window_dims;

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
        pixels: pixel_dims,
        frame_min,
        frame_max,
    });

    let mut framebuffer: Vec<u32> = vec![0; (pixel_dims.x * pixel_dims.y) as usize];

    // Limit to max ~60 fps update rate
    let frame_delay = Duration::from_micros(16_600);
    window.limit_update_rate(Some(frame_delay));

    while window.is_open() {
        if window.is_key_down(Key::Escape) || window.is_key_down(Key::Q) {
            break;
        }

        if window.is_key_down(Key::R) {
            sim.reset();
        }

        // Update as many times as we can within our frame budget.
        // Note: This technically exceeds it is still more flexible than N updates per "frame"
        let mut dur = Duration::new(0, 0);
        while dur < frame_delay {
            let begin = Instant::now();

            sim.update();

            dur += Instant::now() - begin;
        }

        sim.draw(&mut framebuffer);

        if let Err(err) =
            window.update_with_buffer(&framebuffer, pixel_dims.x as usize, pixel_dims.y as usize)
        {
            dbg!(err);
        }
        window.update();
    }
}
