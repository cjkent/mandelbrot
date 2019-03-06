use bmp::Pixel;
use vector3d::Vector3d;

const BLACK: Colour = Colour { r: 0, g: 0, b: 0 };

/// A 24-bit RGB colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Colour {
    pub fn new(r: u8, g: u8, b: u8) -> Colour {
        Colour { r, g, b }
    }

    pub fn from_vector3d(vec3d: &Vector3d) -> Colour {
        Colour::new(vec3d.x as u8, vec3d.y as u8, vec3d.z as u8)
    }

    pub fn from_24bit_int(colour: u32) -> Colour {
        let red = (colour & 0xff0000) >> 16;
        let green = (colour & 0x00ff00) >> 8;
        let blue = colour & 0x0000ff;
        Colour::new(red as u8, green as u8, blue as u8)
    }

    pub fn to_vector3d(&self) -> Vector3d {
        Vector3d::new(self.r as f64, self.g as f64, self.b as f64)
    }

    pub fn pixel(&self) -> Pixel {
        px!(self.r, self.g, self.b)
    }
}

pub fn pixel_colour(
    set: &Vec<u32>,
    real_idx: u32,
    imag_idx: u32,
    width_px: u32,
    oversampling: u32,
    min_iter: u32,
    colours: &Vec<Colour>,
) -> Colour {

    // index of the bottom-left pixel
    let idx_base = (width_px * imag_idx * oversampling * oversampling) + (real_idx * oversampling);
    // use a Vector3d because it's floating point and we need to find the average colour
    // convert back to a Colour at the end
    let mut total_col = Vector3d::new(0.0, 0.0, 0.0);

    for i in 0..oversampling {
        for r in 0..oversampling {
            let idx = idx_base + i * width_px * oversampling + r;
            let iters = set[idx as usize];
            // use the number of iterations to look up the colour in the palette
            // the palette has the right number of colours so each number of iterations
            // is rendered in a different colour
            let col = if iters == 0 {
                BLACK
            } else {
                colours[(iters - min_iter) as usize]
            };
            total_col = total_col + col.to_vector3d();
        }
    }
    let average_col = total_col / ((oversampling * oversampling) as f64);
    Colour::from_vector3d(&average_col)
}

// TODO split some of this out into helper functions so it's easier to test
/// Creates a vector of colours of the specified size defined by the colours in `colours`.
///
/// The colours describe a path through the 3D cube of RGB colours.
pub fn palette(size: u32, colours: &Vec<Colour>) -> Vec<Colour> {
    if colours.len() < 2 {
        panic!("A palette is defined by two or more colours but the size was {}", size);
    }
    if size < colours.len() as u32 {
        panic!("The size of a palette must not be less than the number of colours defining the palette")
    }
    // TODO this is a bad name, it's not the number of colours, it's the number of gaps between them
    let num_cols = size - 1;
    // convert the colours to Vector3d vertices defining the points in the path through the colour cube
    let vertices = colours.iter().map(|col| Vector3d::from_colour(col)).collect::<Vec<_>>();
    // relative vectors from each vertex to the next, 1 element shorter than vertices
    let rel_vecs = relative_vectors(&vertices);
    // divide the size by the number of vertices to get the number of colours per segment
    // each segment has the same number of colours, mostly because that's the easiest to implement
    let num_segs = (colours.len() - 1) as u32;
    let cols_per_seg = num_cols / num_segs;
    let remainder = num_cols % num_segs;
    debug!("num_segs = {}, cols_per_seg = {}, remainder = {}", num_segs, cols_per_seg, remainder);
    // create a vector holding the number of colours per segments. ideally all segments have the
    // same number of colours, but if the number of colours isn't exactly divisible by the
    // number of segments then some segments have one extra colour
    let mut seg_sizes = vec![cols_per_seg; num_segs as usize];

    for i in 0..remainder {
        seg_sizes[i as usize] += 1;
    }
    debug!("seg_sizes = {:?}", seg_sizes);
    // the vector containing the palette
    let mut palette = Vec::with_capacity(num_cols as usize);
    palette.push(colours[0]);

    // loop over each segment
    for i in 0..rel_vecs.len() {
        // the start colour of the leg
        let start = vertices[i];
        // the relative vector from the start colour to the end colour
        let vec = rel_vecs[i];
        // calculate the vector for advancing one colour by dividing the vector by its
        // magnitude and multiplying by the distance between colours
        let col_vec = vec / (cols_per_seg as f64);

        // loop over each point in the segment, adding a colour to the palette
        for c in 1..seg_sizes[i] {
            let col = start + col_vec * (c as f64);
            palette.push(Colour::from_vector3d(&col));
        }
        palette.push(colours[i + 1]);
    }
    palette
}

/// Converts a vector of absolute `Vector3d` instances to a vector of relative `Vector3d` instances.
///
/// The returned vector contains the relative vector from each vertex to the next vertex.
/// Therefore it contains one element less than the input vector.
fn relative_vectors(vertices: &Vec<Vector3d>) -> Vec<Vector3d> {
    if vertices.len() < 2 {
        vec![]
    } else {
        vertices.windows(2).map(|w| w[1] - w[0]).collect()
    }
}

//--------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use vector3d::Vector3d;

    #[test]
    fn relative_vectors_empty() {
        assert!(super::relative_vectors(&vec![]).is_empty());
    }

    #[test]
    fn relative_vectors_one_element() {
        let v = vec![Vector3d::new(1.0, 2.0, 3.0)];
        assert!(super::relative_vectors(&v).is_empty());
    }

    #[test]
    fn relative_vectors_not_empty() {
        let v = vec![
            Vector3d::new(1.0, 2.0, 3.0),
            Vector3d::new(1.0, 3.0, 4.0),
            Vector3d::new(3.0, 2.0, 4.0),
        ];
        let exp = vec![Vector3d::new(0.0, 1.0, 1.0), Vector3d::new(2.0, -1.0, 0.0)];
        assert_eq!(super::relative_vectors(&v), exp);
    }

    #[test]
    fn palette_2_colours_on_axis() {
        let cols = palette(6, &vec![Colour::new(0, 0, 0), Colour::new(255, 0, 0)]);
        let expected = vec![
            Colour::new(0, 0, 0),
            Colour::new(51, 0, 0),
            Colour::new(102, 0, 0),
            Colour::new(153, 0, 0),
            Colour::new(204, 0, 0),
            Colour::new(255, 0, 0),
        ];
        assert_eq!(cols, expected);
    }

    #[test]
    fn palette_2_colours_long_diagonal() {
        let cols = palette(6, &vec![Colour::new(0, 0, 0), Colour::new(255, 255, 255)]);
        let expected = vec![
            Colour::new(0, 0, 0),
            Colour::new(51, 51, 51),
            Colour::new(102, 102, 102),
            Colour::new(153, 153, 153),
            Colour::new(204, 204, 204),
            Colour::new(255, 255, 255),
        ];
        assert_eq!(cols, expected);
    }

    #[test]
    fn palette_3_colours_along_axes() {
        let colours = &vec![
            Colour::new(0, 0, 0),
            Colour::new(255, 0, 0),
            Colour::new(255, 255, 0),
        ];
        let cols = palette(11, colours);
        let expected = vec![
            Colour::new(0, 0, 0),
            Colour::new(51, 0, 0),
            Colour::new(102, 0, 0),
            Colour::new(153, 0, 0),
            Colour::new(204, 0, 0),
            Colour::new(255, 0, 0),
            Colour::new(255, 51, 0),
            Colour::new(255, 102, 0),
            Colour::new(255, 153, 0),
            Colour::new(255, 204, 0),
            Colour::new(255, 255, 0),
        ];
        assert_eq!(cols, expected);
    }
}
