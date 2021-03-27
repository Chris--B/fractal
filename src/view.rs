use minifb::{Key, KeyRepeat, ScaleMode, Window, WindowOptions};
use ultraviolet::{DVec2, UVec2};

use std::time::{Duration, Instant};

use fractal::{make_default_frame, palette, Sim, SimConfig};

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
    // See more frames here:
    // http://www.cuug.ab.ca/dewara/mandelbrot/Mandelbrowser.html

    // "The" Mandelbrot View
    let (frame_min, frame_max) = make_default_frame();
    let window_dims = pick_window_dims(frame_min, frame_max);
    let fb_dims = window_dims;

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
        fb_dims,
        frame_min,
        frame_max,
    });

    let mut framebuffer: Vec<u32> = vec![0; (fb_dims.x * fb_dims.y) as usize];

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

        let color = palette::with_plain_colors;
        // let color = palette::with_plain_colors_smooth ;
        // let color = palette::with_smooth_stripes ;
        // let color = palette::with_lambert_and_colors ;
        // let color = palette::with_white_lambert ;
        sim.draw(&mut framebuffer, color);

        // If we stepped a single frame this loop, reset our state to Paused
        // Otherwise, we'll keep updating!
        if matches!(state, State::RunOneFrame) {
            state = State::Paused;
        }

        // Update the framebuffer unconditionally
        window
            .update_with_buffer(&framebuffer, fb_dims.x as usize, fb_dims.y as usize)
            .unwrap();
    }
}
