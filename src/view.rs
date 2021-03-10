#![allow(dead_code)]

use minifb::{Key, KeyRepeat, ScaleMode, Window, WindowOptions};
use num::Complex;
use ultraviolet::{DVec2, DVec3, UVec2};

#[cfg(feature = "rayon")]
use rayon::prelude::*;

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
    dc: Complex<f64>,
    dz: Complex<f64>,

    iters: u32,
    has_escaped: bool,
}

impl GridCell {
    fn new(c: Complex<f64>) -> Self {
        GridCell {
            c,
            z: Complex::new(0., 0.),
            dc: Complex::new(1., 0.),
            dz: Complex::new(1., 0.),

            iters: 0,
            has_escaped: false,
        }
    }

    fn step(&mut self) {
        // Skip if we've already escaped
        if self.has_escaped {
            return;
        }

        // Perform our iteration
        self.iters += 1;

        // Copy values out so we can update them
        let GridCell { c, z, dc, dz, .. } = *self;

        self.z = z * z + c;
        self.dz = dz * 2. * z + dc;

        // Check our typical escape condition
        if self.z.norm_sqr() > 4.0 {
            self.has_escaped = true;
        }
    }
}

// Use a color palette that cycles based off of iterations
// Sourced from StackOverflow: https://stackoverflow.com/a/16505538
const COLOR_MAPPING: [DVec3; 16] = [
    DVec3::new(66., 30., 15.),
    DVec3::new(25., 7., 26.),
    DVec3::new(9., 1., 47.),
    DVec3::new(4., 4., 73.),
    DVec3::new(0., 7., 100.),
    DVec3::new(12., 44., 138.),
    DVec3::new(24., 82., 177.),
    DVec3::new(57., 125., 209.),
    DVec3::new(134., 181., 229.),
    DVec3::new(211., 236., 248.),
    DVec3::new(241., 233., 191.),
    DVec3::new(248., 201., 95.),
    DVec3::new(255., 170., 0.),
    DVec3::new(204., 128., 0.),
    DVec3::new(153., 87., 0.),
    DVec3::new(106., 52., 3.),
];

fn palette_with_plain_colors(cell: &GridCell) -> DVec3 {
    if cell.has_escaped {
        // Color from iterations
        COLOR_MAPPING[cell.iters as usize % COLOR_MAPPING.len()] / 255.
    } else {
        DVec3::broadcast(0.)
    }
}

fn palette_with_lambert_and_colors(cell: &GridCell) -> DVec3 {
    let color = if cell.has_escaped {
        // Color from iterations
        COLOR_MAPPING[cell.iters as usize % COLOR_MAPPING.len()] / 255.
    } else {
        0.8 * DVec3::new(205., 92., 92.) / 255.
    };

    // Normal of the "surface"
    let u: Complex<_> = cell.z / cell.dz;
    let u = DVec2::new(u.re, u.im).normalized();
    let n = DVec3::new(u.x, u.y, 1.);

    // Our point's location
    let pos = DVec3::new(cell.c.re, cell.c.im, 0.);

    // Light source
    const L_POS: DVec3 = DVec3::new(-2.1, 0.75, 4.);
    let l_dir = (L_POS - pos).normalized();
    let t = 0.75 / l_dir.mag();
    let t = t.max(0.0);

    t * n.dot(l_dir).max(0.0) * color
}

fn palette_with_white_lambert(cell: &GridCell) -> DVec3 {
    let color = if cell.has_escaped {
        DVec3::new(1., 1., 1.)
    } else {
        // If we haven't escaped yet, use black
        DVec3::new(0., 0., 0.)
    };

    // Normal of the "surface"
    let u: Complex<_> = cell.z / cell.dz;
    let u = DVec2::new(u.re, u.im).normalized();
    let n = DVec3::new(u.x, u.y, 1.);

    // Our point's location
    let pos = DVec3::new(cell.c.re, cell.c.im, 0.);

    // Light source
    const L_POS: DVec3 = DVec3::new(-2.1, 0.75, 4.);
    let l_dir = (L_POS - pos).normalized();
    let t = 0.75 / l_dir.mag();
    let t = t.max(0.0);

    t * n.dot(l_dir).max(0.0) * color
}

struct Sim {
    config: SimConfig,
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

        Self { config, grid }
    }

    /// Reset the sim state to a fresh object
    fn reset(&mut self) {
        self.grid.clear();

        let framebuffer_size = self.config.pixels.x * self.config.pixels.y;
        for idx in 0..framebuffer_size {
            let c: Complex<_> = self.config.idx_to_complex(idx);
            self.grid.push(GridCell::new(c));
        }
    }

    fn update(&mut self) {
        #[cfg(feature = "rayon")]
        {
            self.grid
                .par_iter_mut()
                // Skip already diverged cells
                .filter(|c| !c.has_escaped)
                .for_each(|cell| {
                    cell.step();
                })
        }

        #[cfg(not(feature = "rayon"))]
        {
            for cell in self.grid.iter_mut().filter(|c| !c.has_escaped) {
                cell.step();
            }
        }
    }

    fn draw(&mut self, fb: &mut [u32]) {
        assert_eq!(fb.len(), self.grid.len());

        // ==== Pick your color function at compile time!
        use palette_with_plain_colors as color;
        // use palette_with_lambert_and_colors as color;
        // use palette_with_white_lambert as color;

        #[cfg(feature = "rayon")]
        {
            fb.par_iter_mut().enumerate().for_each(|(i, pixel)| {
                let mut c = color(&self.grid[i]);
                // Clamp and scale all output from `color` into the range for our 8-bit channels: [0, 255]
                c.clamp(DVec3::new(0., 0., 0.), DVec3::new(1., 1., 1.));
                c *= 255.;

                *pixel = rgb(c.x as u8, c.y as u8, c.z as u8);
            });
        }

        #[cfg(not(feature = "rayon"))]
        {
            for (i, pixel) in fb.iter_mut().enumerate() {
                let mut c = color(&self.grid[i]);
                // Clamp and scale all output from `color` into the range for our 8-bit channels: [0, 255]
                c.clamp(DVec3::new(0., 0., 0.), DVec3::new(1., 1., 1.));
                c *= 255.;

                *pixel = rgb(c.x as u8, c.y as u8, c.z as u8);
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

    // Limit to max ~60 fps update rate
    let frame_delay = Duration::from_micros(16_600);
    window.limit_update_rate(Some(frame_delay));

    // Limit how quickly holding the Arrow Key down sends us updated events
    window.set_key_repeat_delay(0.2);
    window.set_key_repeat_rate(0.2);

    let mut sim = Sim::new(SimConfig {
        pixels: pixel_dims,
        frame_min,
        frame_max,
    });

    let mut framebuffer: Vec<u32> = vec![0; (pixel_dims.x * pixel_dims.y) as usize];

    #[derive(Copy, Clone, Debug)]
    enum State {
        Paused,
        Running,
        RunOneFrame,
    }

    let mut state = State::Running;

    while window.is_open() {
        // Keys to quit
        if window.is_key_down(Key::Escape) || window.is_key_down(Key::Q) {
            break;
        }

        // Reset the simulation state
        if window.is_key_down(Key::R) {
            sim.reset();
        }

        // Toggle Pause
        if window.is_key_pressed(Key::Space, KeyRepeat::No) {
            if matches!(state, State::Paused) {
                state = State::Running;
            } else {
                state = State::Paused;
            }
        }

        // Advance one iteration at a time with the Right Arrow key
        if window.is_key_pressed(Key::Right, KeyRepeat::Yes) {
            if matches!(state, State::Paused) {
                state = State::RunOneFrame;
            }
        }

        // Run (or don't run) the simulation
        match state {
            State::Paused => {
                // Nothing to do when paused
            }
            State::Running => {
                // Update as many times as we can within our frame budget.
                let mut estimate = {
                    let begin = Instant::now();
                    sim.update();
                    Instant::now() - begin
                };

                let mut left = frame_delay;
                while left > estimate {
                    let begin = Instant::now();
                    sim.update();

                    let dur = Instant::now() - begin;
                    estimate = estimate.max(dur);

                    // Duration panics on underflow, so check it here
                    if left > dur {
                        left -= dur;
                    } else {
                        break;
                    }
                }
            }
            State::RunOneFrame => {
                // Time and run a single frame
                let begin = Instant::now();
                sim.update();
                let dur = Instant::now() - begin;

                println!("sim.update() took {:?}", dur);
            }
        }

        // Re-draw on the framebuffer unconditionally
        sim.draw(&mut framebuffer);

        // If we stepped a single frame this loop, reset our state to Paused
        // Otherwise, we'll keep updating!
        if matches!(state, State::RunOneFrame) {
            state = State::Paused;
        }

        // Update the framebuffer unconditionally
        window
            .update_with_buffer(&framebuffer, pixel_dims.x as usize, pixel_dims.y as usize)
            .unwrap();
    }
}
