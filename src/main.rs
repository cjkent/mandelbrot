#[macro_use]
extern crate log;
extern crate env_logger;
extern crate time;
#[macro_use]
extern crate bmp;
extern crate threadpool;

mod complex;
mod colour;
mod vector3d;

use threadpool::ThreadPool;
use std::sync::mpsc::channel;
use complex::Complex;
use std::vec::Vec;
use bmp::Image;
use colour::Colour;
use std::sync::mpsc;
use std::error::Error;

fn main() {
    env_logger::init().unwrap();
    let start_time = time::precise_time_s();
    let set_def = SetDefinition::new(-0.77, -0.74, 0.07, 0.11, 1200, 2, 400, 10.0);
//    let set_def = SetDefinition::new(-2.0, 1.0, -1.0, 1.0, 1200, 2, 100, 10.0);
    info!("set_def = {:?}", set_def);
    // TODO use std::env::args() or getopts to specify the number of threads?
    let set_data = calc_set_parallel(&set_def, 8);
//    let set_data = calc_set(&set_def);
    info!("time taken to calculate set {:.*}ms", 2, (time::precise_time_s() - start_time) * 1000f64);
    info!("set_data size = {}", set_data.data.len());
    let img = render(&set_data);
    let _ = img.save("/Users/cj/tmp/mandelbrot.bmp");
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SetDefinition {
    origin: Complex,
    px_size: f64,
    width_px: u32,
    height_px: u32,
    oversampling: u32,
    max_iterations: u32,
    escape_radius: f64,
}

/// Definition which specifies how to calculate the Mandelbrot Set for an area of
/// the complex plane.
impl SetDefinition {

    // TODO Replace this with a builder
    fn new(min_real: f64,
           max_real: f64,
           min_imag: f64,
           max_imag: f64,
           width_px: u32,
           oversampling: u32,
           max_iterations: u32,
           escape_radius: f64) -> SetDefinition {

        // TODO validate min and max values are different and in the correct order

        let px_size = (max_real - min_real) / (width_px as f64);
        let height_px = (max_imag - min_imag) / px_size;

        SetDefinition {
            origin: Complex::new(min_real, min_imag),
            px_size: px_size,
            width_px: width_px,
            height_px: (height_px as u32),
            oversampling: oversampling,
            max_iterations: max_iterations,
            escape_radius: escape_radius,
        }
    }

    /// Splits this definition into multiple definitions covering the same area,
    /// allowing them to be processed in parallel and assembled into a single image
    /// during rendering.
    ///
    /// The area is split into evenly sized horizontal strips. Each area in the returned vector
    /// has the same width as the input and has a height approximately equal to the height of
    /// the input divided by `count`.
    fn split(&self, count: u32) -> Vec<SetDefinition> {
        let mut heights = vec![self.height_px / count; count as usize];
        let rem = self.height_px % count;

        for i in 0..rem {
            heights[i as usize] += 1;
        }
        let mut imag = self.origin.imag;
        let mut defs: Vec<SetDefinition> = Vec::with_capacity(count as usize);

        for i in 0..count {
            let origin = Complex::new(self.origin.real, imag);
            let height = heights[i as usize];
            let def = SetDefinition { origin: origin, height_px: height, .. *self };
            defs.push(def);
            imag += height as f64 * self.px_size;
        }
        defs
    }
}

struct SetData {
    def: SetDefinition,
    data: Vec<u32>,
}

//--------------------------------------------------------------------------------------------------

/// Returns the number of iterations it takes the point's magnitude to exceed the
/// escape radius. Zero is returned if the point is in the set.
fn escape_iterations(point: Complex, max_iterations: u32, escape_radius: f64) -> u32 {
    let escape_value = escape_radius * escape_radius;
    let mut z = point;

    for i in 0..max_iterations {
        // it's more efficient to explode the complex into real and imaginary parts rather
        // than multiplying the Complex. this way the squares only need to be calculated once
        // and the square root can be avoided altogether
        let zr2 = z.real * z.real;
        let zi2 = z.imag * z.imag;
        let zri = z.real * z.imag;

        if zr2 + zi2 > escape_value {
            return i
        }
        z = Complex::new(zr2 - zi2 + point.real, zri + zri + point.imag);
    }
    0
}

//fn escape_iterations_simd(point1: Complex,
//                          point2: Complex,
//                          max_iterations: u32,
//                          escape_radius: f64) -> (u32, u32) {
//
//    let escape_value = f64x2::splat(escape_radius * escape_radius);
//    let mut real = f64x2::new(point1.real, point2.real);
//    let mut imag = f64x2::new(point1.imag, point2.imag);
//    let mut iter_count = u64x2::splat(0.0);
//
//    for _ in 0..max_iterations {
//        // it's more efficient to explode the complex into real and imaginary parts rather
//        // than multiplying the Complex. this way the squares only need to be calculated once
//        // and the square root can be avoided altogether
//        let zr2 = real * real;
//        let zi2 = imag * imag;
//        let zri = real * imag;
//        let mask = (zr2 + zi2).gt(escape_value);
//
//        if mask.all() {
//            return
//        }
//        z = Complex::new(zr2 - zi2 + point.real, zri + zri + point.imag);
//    }
//    0
//}

/// Calculates a set in parallel using the thread pool.
fn calc_set_parallel(set_def: &SetDefinition, threads: u32) -> SetData {
    let thread_pool = ThreadPool::new(threads as usize);
    let (tx, rx) = mpsc::channel();
    // TODO What multiplier?
    let defs = set_def.split(threads * 10);
    let size = defs.len();

    for (idx, def) in defs.into_iter().enumerate() {
        let tx_clone = tx.clone();
        thread_pool.execute(move || {
            let set_data = calc_set(&def);
            // send back a tuple with the index and the calculated set data
            // the index allows the sets to be assembled in the correct order to create an image
            tx_clone.send((idx, set_data)).unwrap();
        });
    }
    // vector containing pairs of (index, SetData), each element is one slice of the whole set
    let mut sets: Vec<(usize, SetData)> = Vec::with_capacity(size);

    // fill up the vector with values sent over channels from the threads calculating the sets
    while sets.len() < size {
        match rx.recv() {
            Ok(data) => sets.push(data),
            Err(err) => panic!("Received error '{}'", err.description()),
        };
    }
    // sort the sets by index so the strips are in the correct order before rendering
    sets.sort_by(|&(idx1, _), &(idx2, _)| idx1.cmp(&idx2));
    // create a vector containing only the set data, not the indices
    let mut data_vec = sets.into_iter().map(|(_, set_data)| set_data.data).collect::<Vec<_>>();
    let capacity = set_def.width_px * set_def.height_px;
    // create a vector to hold the data for the entire set
    let mut data = Vec::with_capacity(capacity as usize);

    for v in data_vec.iter_mut() {
        data.append(v);
    }
    SetData { def: *set_def, data: data }
}

/// Calculates the set defined by `set_def`.
fn calc_set(set_def: &SetDefinition) -> SetData {
    let capacity = set_def.width_px * set_def.height_px * set_def.oversampling * set_def.oversampling;
    let mut point_data: Vec<u32> = Vec::with_capacity(capacity as usize);
    let px_size = set_def.px_size / (set_def.oversampling as f64);

    for i in 0..set_def.height_px * set_def.oversampling {
        for r in 0..set_def.width_px * set_def.oversampling {
            let point = set_def.origin + Complex::new((r as f64) * px_size, (i as f64) * px_size);
            let escape_iters = escape_iterations(point, set_def.max_iterations, set_def.escape_radius);
            point_data.push(escape_iters);
        }
    }
    SetData { def: *set_def, data: point_data }
}

/// Renders Mandelbrot Set data into an image.
fn render(set: &SetData) -> Image {
    let mut img = Image::new(set.def.width_px, set.def.height_px);
    // TODO This needs to handle set data calculated in parallel
    let (min_iter, max_iter) = escape_iter_range(&set.data);
    info!("(min_iter, max_iter) = ({}, {})", min_iter, max_iter);
    // TODO Need to create a fixed, larger number of colours and smooth between iterations.
    let num_colours = max_iter - min_iter + 1;
    debug!("num_colours = {}", num_colours);
    let palette_vertices = vec![
        Colour::from_24bit_int(0x010d62),
        Colour::from_24bit_int(0x63b8ec),
        Colour::from_24bit_int(0xffffff),
        Colour::from_24bit_int(0xffb700),
        Colour::from_24bit_int(0x611012),
    ];
    let colours = colour::palette(num_colours, &palette_vertices);
    debug!("colours.len() = {}", colours.len());

    for (x, y) in img.coordinates() {
        let real_idx = x;
        // need to reverse the y co-ordinate because the image origin is top left
        let imag_idx = set.def.height_px - y - 1;
        let clr = colour::pixel_colour(&set.data,
                                       real_idx,
                                       imag_idx,
                                       set.def.width_px,
                                       set.def.oversampling,
                                       min_iter,
                                       &colours);
        img.set_pixel(x, y, clr.pixel());
    }
    img
}

fn escape_iter_range(set_vec: &Vec<u32>) -> (u32, u32) {
    let mut min = set_vec[0];
    let mut max = set_vec[0];

    for i in 1..set_vec.len() {
        let val = set_vec[i];

        if val > max {
            max = val;
        }
        if val < min {
            min = val;
        }
    }
    (min, max)
}

//--------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::SetDefinition;
    use complex::Complex;

    #[test]
    fn split_simple() {
        let def = SetDefinition {
            origin: Complex::new(1.0, 2.0),
            px_size: 0.01,
            width_px: 200,
            height_px: 100,
            oversampling: 2,
            max_iterations: 100,
            escape_radius: 2.0,
        };
        let expected = vec![
            SetDefinition { origin: Complex::new(1.0, 2.0), height_px: 25, .. def },
            SetDefinition { origin: Complex::new(1.0, 2.25), height_px: 25, .. def },
            SetDefinition { origin: Complex::new(1.0, 2.5), height_px: 25, .. def },
            SetDefinition { origin: Complex::new(1.0, 2.75), height_px: 25, .. def },
        ];
        assert_eq!(def.split(4), expected);
    }

    #[test]
    fn split_with_remainder() {
        let def = SetDefinition {
            origin: Complex::new(1.0, 2.0),
            px_size: 0.01,
            width_px: 200,
            height_px: 100,
            oversampling: 2,
            max_iterations: 100,
            escape_radius: 2.0,
        };
        let expected = vec![
            SetDefinition { origin: Complex::new(1.0, 2.0), height_px: 34, .. def },
            SetDefinition { origin: Complex::new(1.0, 2.34), height_px: 33, .. def },
            SetDefinition { origin: Complex::new(1.0, 2.67), height_px: 33, .. def },
        ];
        assert_eq!(def.split(3), expected);
    }
}
