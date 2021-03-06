use minifb::{Key, Window, WindowOptions};

#[inline]
fn rand_color() -> u32 {
    use rand::RngCore;

    const OPAQUE: u32 = 0xFF_00_00_00;

    rand::thread_rng().next_u32() | OPAQUE
}

struct Sim {
    px_dim_x: usize,
    px_dim_y: usize,

    bounds_min_x: f64,
    bounds_min_y: f64,
    bounds_max_x: f64,
    bounds_max_y: f64,
}

impl Sim {
    fn draw(&mut self, fb: &mut [u32]) {
        for i in fb.iter_mut() {
            *i = rand_color();
        }
    }
}

fn main() {
    const HEIGHT: usize = 780;
    const WIDTH: usize = (HEIGHT as f64 * (1.0 - -2.0) / (1.0 - -1.0)) as usize;

    dbg!(WIDTH);

    let mut framebuffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let mut window = Window::new(
        "Test - ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    // Limit to max ~60 fps update rate
    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    let mut sim = Sim {
        px_dim_x: WIDTH,
        px_dim_y: HEIGHT,

        bounds_min_x: -2.0,
        bounds_max_x: 1.0,
        bounds_min_y: -1.0,
        bounds_max_y: 1.0,
    };

    while window.is_open() {
        if window.is_key_down(Key::Escape) || window.is_key_down(Key::Q) {
            break;
        }

        sim.draw(&mut framebuffer);

        if let Err(err) = window.update_with_buffer(&framebuffer, WIDTH, HEIGHT) {
            dbg!(err);
        }
    }
}
