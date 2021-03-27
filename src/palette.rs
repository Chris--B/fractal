use std::f64::consts::TAU;

use num::Complex;
use ultraviolet::{DVec2, DVec3};

use crate::GridCell;
use crate::R2;

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

pub fn with_plain_colors(cell: &GridCell) -> DVec3 {
    if cell.has_escaped {
        // Color from iterations
        COLOR_MAPPING[cell.iters as usize % COLOR_MAPPING.len()] / 255.
    } else {
        DVec3::broadcast(0.)
    }
}

pub fn with_smooth_stripes(cell: &GridCell) -> DVec3 {
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

pub fn with_lambert_and_colors(cell: &GridCell) -> DVec3 {
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

pub fn with_white_lambert(cell: &GridCell) -> DVec3 {
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

pub fn with_color_from_dz(cell: &GridCell) -> DVec3 {
    let x = 30. * cell.dz.re;

    // Color from the derivative of z
    // This does not distinguish between escaped or not, but dz relates to this anyway, so
    // it's still visible in the final image.
    COLOR_MAPPING[x as usize % COLOR_MAPPING.len()] / 255.
}
