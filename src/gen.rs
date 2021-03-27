use ultraviolet::UVec2;

use std::time::{Duration, Instant};

use fractal::{make_default_frame, palette, Sim, SimConfig};

fn main() {
    // See more frames here:
    // http://www.cuug.ab.ca/dewara/mandelbrot/Mandelbrowser.html

    // "The" Mandelbrot View
    let (frame_min, frame_max) = make_default_frame();
    let aspect_ratio = (frame_max.x - frame_min.x) as f64 / (frame_max.y - frame_min.y) as f64;

    let width = 1080.;
    let height = width / aspect_ratio;
    let fb_dims = UVec2::new(width as u32, height as u32);

    let mut sim = Sim::new(SimConfig {
        fb_dims,
        frame_min,
        frame_max,
    });

    let mut step_times: Vec<Duration> = vec![];
    let raw_begin = Instant::now();
    let filename = format!("mandelbrot-{}x{}.png", fb_dims.x, fb_dims.y);
    println!("Rendering {}", filename);

    // TODO: How do we know when we're done....?
    let steps = 1_000;
    for _ in 0..steps {
        let begin = Instant::now();

        sim.update();

        let end = Instant::now();
        step_times.push(end - begin);
    }

    let raw_end = Instant::now();

    // Print stats

    let wall = raw_end - raw_begin;
    let sum: Duration = step_times.iter().sum();
    let ave = {
        let ns = sum.as_nanos() as f64;
        let ave = ns / step_times.len() as f64;

        Duration::from_nanos(ave as u64)
    };
    let overhead = wall - sum;

    dbg!(wall);
    dbg!(steps);
    dbg!(sum);
    dbg!(ave);
    dbg!(overhead);

    // Render and write out image
    let mut framebuffer: Vec<u32> = vec![0; (fb_dims.x * fb_dims.y) as usize];

    let color = palette::with_plain_colors;
    // let color = palette::with_plain_colors_smooth ;
    // let color = palette::with_smooth_stripes ;
    // let color = palette::with_lambert_and_colors ;
    // let color = palette::with_white_lambert ;
    sim.draw(&mut framebuffer, color);

    // Change format from 0RGB -> to RGBA, both 8-bit channels
    // We'll always use 0xFF for alpha.
    const A: u8 = 0xff;
    for px in framebuffer.iter_mut() {
        // Each pixel is encoded as 0RGB
        let [z, r, g, b] = px.to_be_bytes();
        assert_eq!(z, 0);

        // Re-encode as RGBA
        *px = u32::from_le_bytes([r, g, b, A]);
    }

    image::save_buffer(
        filename,
        bytemuck::cast_slice(&framebuffer),
        fb_dims.x,
        fb_dims.y,
        image::ColorType::Rgba8,
    )
    .expect("Failed to save image");
}
