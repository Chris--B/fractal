use num::Complex;
use ultraviolet::{DVec2, DVec3, UVec2};

#[cfg(feature = "rayon")]
use rayon::prelude::*;

use std::f64::consts::TAU;

/// Construct a color for use with minifb
///
/// The encoding for each pixel is 0RGB
const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    let (r, g, b) = (r as u32, g as u32, b as u32);

    (r << 16) | (g << 8) | b
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimConfig {
    /// 2D Dimensions of the framebuffer
    pub fb_dims: UVec2,

    /// Complex point of the lower-left (-x & -y) point of the frame
    pub frame_min: DVec2,

    /// Complex point of the upper-right (+x & +y) point of the frame
    pub frame_max: DVec2,
}

impl SimConfig {
    #[inline]
    fn idx_to_complex(&self, idx: u32) -> Complex<f64> {
        // Unpack out integer coordinates
        let x = idx % self.fb_dims.x;
        let y = idx / self.fb_dims.x;

        // Normalize coordinates
        let x: f64 = (x as f64) / (self.fb_dims.x as f64);
        let y: f64 = (y as f64) / (self.fb_dims.y as f64);

        // Flip the buffer to put "bigger" y at the "top"
        let y: f64 = 1.0 - y;

        // Scale into the bounds space
        let x = x * self.frame_max.x + (1.0 - x) * self.frame_min.x;
        let y = y * self.frame_max.y + (1.0 - y) * self.frame_min.y;

        Complex::new(x, y)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct GridCell {
    pub c: Complex<f64>,
    pub z: Complex<f64>,
    pub dc: Complex<f64>,
    pub dz: Complex<f64>,

    pub iters: u32,
    pub has_escaped: bool,
}

const R2: u32 = 1_000 * 1_000;

impl GridCell {
    pub fn new(c: Complex<f64>) -> Self {
        GridCell {
            c,
            z: Complex::new(0., 0.),
            dc: Complex::new(1., 0.),
            dz: Complex::new(1., 0.),

            iters: 0,
            has_escaped: false,
        }
    }

    pub fn step(&mut self) {
        // Use a separate threshold for when to stop stepping.
        // This is generally much larger than |2|, but produces better coloring schemes.
        if self.z.norm_sqr() > R2 as f64 {
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

pub fn palette_with_plain_colors(cell: &GridCell) -> DVec3 {
    if cell.has_escaped {
        // Color from iterations
        COLOR_MAPPING[cell.iters as usize % COLOR_MAPPING.len()] / 255.
    } else {
        DVec3::broadcast(0.)
    }
}

pub fn palette_with_smooth_stripes(cell: &GridCell) -> DVec3 {
    fn f(x: f64) -> DVec3 {
        let c = (1. + f64::cos(TAU * x)) / 2.;
        DVec3::broadcast(c)
    }

    let z2 = cell.z.norm_sqr();
    if z2 > R2 as f64 {
        let v: f64 = f64::log2(z2) / f64::powf(2., cell.iters as f64);
        f(v.log2())
    } else {
        DVec3::broadcast(1.)
    }
}

pub fn palette_with_lambert_and_colors(cell: &GridCell) -> DVec3 {
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

pub fn palette_with_white_lambert(cell: &GridCell) -> DVec3 {
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

pub struct Sim {
    config: SimConfig,
    grid: Vec<GridCell>,
}

impl Sim {
    pub fn new(config: SimConfig) -> Self {
        let framebuffer_size = config.fb_dims.x * config.fb_dims.y;
        let mut grid = Vec::with_capacity(framebuffer_size as usize);

        for idx in 0..framebuffer_size {
            let c = config.idx_to_complex(idx);
            grid.push(GridCell::new(c));
        }

        assert_eq!(grid.len(), framebuffer_size as usize);

        Self { config, grid }
    }

    /// Reset the sim state to a fresh object
    pub fn reset(&mut self) {
        self.grid.clear();

        let framebuffer_size = self.config.fb_dims.x * self.config.fb_dims.y;
        for idx in 0..framebuffer_size {
            let c: Complex<_> = self.config.idx_to_complex(idx);
            self.grid.push(GridCell::new(c));
        }
    }

    pub fn update(&mut self) {
        #[cfg(feature = "rayon")]
        {
            self.grid.par_iter_mut().for_each(|cell| {
                cell.step();
            })
        }

        #[cfg(not(feature = "rayon"))]
        {
            for cell in self.grid.iter_mut() {
                cell.step();
            }
        }
    }

    pub fn draw(&mut self, fb: &mut [u32]) {
        assert_eq!(fb.len(), self.grid.len());

        // ==== Pick your color function at compile time!
        use palette_with_plain_colors as color;
        // use palette_with_plain_colors_smooth as color;
        // use palette_with_smooth_stripes as color;
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

/// Make a square frame centered at `p` with radius `r`
pub fn make_square_frame(p: DVec2, r: f64) -> (DVec2, DVec2) {
    let min: DVec2 = DVec2::new(p.x - r, p.y - r);
    let max: DVec2 = DVec2::new(p.x + r, p.y + r);

    (min, max)
}

/// Makes a frame that shows "The" Mandelbrot fractal how everyone expects it
pub fn make_default_frame() -> (DVec2, DVec2) {
    let min: DVec2 = DVec2::new(-2.5, -1.25);
    let max: DVec2 = DVec2::new(1.0, 1.25);

    (min, max)
}
