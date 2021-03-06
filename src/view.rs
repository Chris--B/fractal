#![allow(dead_code)]

use minifb::{Key, Window, WindowOptions};
use num::Complex;

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

struct Sim {
    px_dim_x: u32,
    px_dim_y: u32,

    bounds_min_x: f64,
    bounds_min_y: f64,
    bounds_max_x: f64,
    bounds_max_y: f64,
}

impl Sim {
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

fn main() {
    // Visually appealing framing for the Mandelbrot set
    const BOUNDS_X: (f64, f64) = (-2.5, 1.0);
    const BOUNDS_Y: (f64, f64) = (-1.25, 1.25);

    // Scale the window with our bounds
    // TODO: Implement a max so it stays on-screen
    const HEIGHT: usize = 780;
    const WIDTH: usize =
        (HEIGHT as f64 * (BOUNDS_X.1 - BOUNDS_X.0) / (BOUNDS_Y.1 - BOUNDS_Y.0)) as usize;

    let mut window = Window::new(
        "Test - ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    let mut sim = Sim {
        px_dim_x: WIDTH as u32,
        px_dim_y: HEIGHT as u32,

        bounds_min_x: BOUNDS_X.0,
        bounds_max_x: BOUNDS_X.1,
        bounds_min_y: BOUNDS_Y.0,
        bounds_max_y: BOUNDS_Y.1,
    };

    let mut framebuffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    // Render once
    // This keeps the window interactive once the image has been rendered.
    sim.draw(&mut framebuffer);

    // Limit to max ~60 fps update rate
    window.limit_update_rate(Some(std::time::Duration::from_micros(16_600)));

    while window.is_open() {
        if window.is_key_down(Key::Escape) || window.is_key_down(Key::Q) {
            break;
        }

        if let Err(err) = window.update_with_buffer(&framebuffer, WIDTH, HEIGHT) {
            dbg!(err);
        }
    }
}
