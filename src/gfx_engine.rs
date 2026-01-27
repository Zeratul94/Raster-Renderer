#![allow(unused_must_use)]
#![allow(dead_code)]

extern crate sdl3;
extern crate glam;

use glam::Mat4;
use glam::Vec3;
use sdl3::pixels::PixelFormat;
use sdl3::render::FPoint;
use sdl3::video::WindowContext;

/* Structs */

#[derive(Clone, Copy)]
pub struct Material {
    pub color: sdl3::pixels::Color
}

pub struct Surface<'a> {
    pub width: u16,
    pub height: u16,
    px_buf: Vec<u8>,
    pixel_format: PixelFormat,
    pub render_tex: sdl3::render::Texture<'a>
}

/* Implementations */

impl Material {
    pub fn new(col: sdl3::pixels::Color) -> Material {
        Material { color: col }
    }

    pub const DEFAULT: Self = Self { color: sdl3::pixels::Color { r: 128, g: 128, b: 128, a: 255 } };
}
impl std::fmt::Display for Material {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        return write!(f, "{}, {}, {}", self.color.r, self.color.g, self.color.b);
    }
}

impl<'a> Surface<'a> {

    pub fn new(target_width: u16, target_height: u16, pixelformat: PixelFormat, texture_creator: &'a sdl3::render::TextureCreator<WindowContext>) -> Self {
        let px_buf = vec![0 as u8; pixelformat.byte_size_from_pitch_and_height(target_width as usize * pixelformat.bytes_per_pixel(), target_height as usize)];
        let render_tex: sdl3::render::Texture<'a> = texture_creator.create_texture(pixelformat, sdl3::render::TextureAccess::Streaming, target_width as u32, target_height as u32).unwrap();
        
        Self { width: target_width, height: target_height, px_buf, pixel_format: pixelformat, render_tex }
    }

    // Returns true if the input triangle is wholly or partially within
    // the target width and height. False otherwise
    pub fn clip_tri_to_screen(&self, screen_tri: [FPoint; 3]) -> bool {
        return (screen_tri[0].x >= 0. || screen_tri[1].x >= 0. || screen_tri[2].x >= 0.) // If any of the projected vertices are actually on-screen (should be redundant
        && (screen_tri[0].x <= self.width as f32 || screen_tri[1].x <= self.width as f32 || screen_tri[2].x <= self.width as f32) // with frustum culling)
        && (screen_tri[0].y >= 0. || screen_tri[1].y >= 0. || screen_tri[2].y >= 0.)
        && (screen_tri[0].y <= self.height as f32 || screen_tri[1].y <= self.height as f32 || screen_tri[2].y <= self.height as f32)
    }

    // Draw the input screen triangles to this Surface's render target
    // To display the render target to the screen, use:
    //      your_canvas.copy(&this_surface.render_tex, your_src_or_None, your_dst_or_None);
    pub fn render_tris(&mut self, tris: &mut Vec<([FPoint; 3], f32, usize)>, materials: &Vec<Material>) {
        // Rasterize the projected triangles, and bake them to a texture
        let mut drawndepths = vec![f32::MAX; self.width as usize * self.height as usize];
        self.px_buf.fill(0);
        let l = tris.len();
        tris.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap()); // Sort triangles front-to-back

        let bytes_per_pixel = self.pixel_format.bytes_per_pixel() as usize;
        let masks = self.pixel_format.into_masks().unwrap();

        // Find which byte (0, 1, 2, or 3) corresponds to which color channel. Gemini wrote this.
        let mut r_offset = None;
        let mut g_offset = None;
        let mut b_offset = None;
        let mut a_offset = None;

        for subidx in 0..bytes_per_pixel {
            let shift = subidx * 8;
            if (masks.rmask >> shift & 0xFF) != 0 { r_offset = Some(subidx); }
            if (masks.gmask >> shift & 0xFF) != 0 { g_offset = Some(subidx); }
            if (masks.bmask >> shift & 0xFF) != 0 { b_offset = Some(subidx); }
            if (masks.amask >> shift & 0xFF) != 0 { a_offset = Some(subidx); }
        }

        for i in 0..l {
            self.draw_points_in_triangle(tris[i], &mut drawndepths, bytes_per_pixel, [r_offset, g_offset, b_offset, a_offset], materials);
        }

        // Write to the texture
        self.render_tex.update(None, &self.px_buf, self.width as usize * self.pixel_format.bytes_per_pixel() as usize).unwrap();
    }

    fn draw_points_in_triangle(&mut self, tri_data: ([FPoint; 3], f32, usize), drawndepths: &mut Vec<f32>, bytes_per_pixel: usize, channel_offsets: [Option<usize>; 4], materials: &Vec<Material>) {
        let tri = tri_data.0;
            let clr = materials[tri_data.2].color;
        let min_x = f32::min(f32::min(tri[0].x, tri[1].x), tri[2].x);
        let max_x = f32::max(f32::max(tri[0].x, tri[1].x), tri[2].x);
        let min_y = f32::min(f32::min(tri[0].y, tri[1].y), tri[2].y);
        let max_y = f32::max(f32::max(tri[0].y, tri[1].y), tri[2].y);

        // Clamp to screen bounds (Gemini wrote this little snippet)
        let start_x = (min_x.max(0.0).floor() as u16).min(self.width);
        let end_x   = (max_x.min(self.width as f32 - 1.0).ceil() as u16).min(self.width);
        let start_y = (min_y.max(0.0).floor() as u16).min(self.height);
        let end_y   = (max_y.min(self.height as f32 - 1.0).ceil() as u16).min(self.height);

        // Get the pixels contained by the triangle
        let mut frag = Vec::new();
        if end_y > start_y && end_x > start_x {
            //let mut printed_yet = false; //DEBUG
            for i in 0..=(end_y-start_y) as i32 {
                for j in 0..=(end_x-start_x) as i32 {
                    // Check if the point (j, i) is inside the triangle using Sebastian Lague's vector "point on the right side" method
                    let v0 = Vec3::new(tri[2].x - tri[0].x, tri[2].y - tri[0].y, 0.);
                    let v1 = Vec3::new(tri[1].x - tri[0].x, tri[1].y - tri[0].y, 0.);
                    let v2 = Vec3::new(j as f32 + min_x - tri[0].x, i as f32 + min_y - tri[0].y, 0.);
                    let dot00 = v0.dot(v0);
                    let dot01 = v0.dot(v1);
                    let dot02 = v0.dot(v2);
                    let dot11 = v1.dot(v1);
                    let dot12 = v1.dot(v2);
                    let inv_denom = 1. / (dot00 * dot11 - dot01 * dot01);
                    let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
                    let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;
                    //if !printed_yet {println!("math!"); printed_yet = true;} //DEBUG

                    // If u and v are both between 0 and 1, the point is inside the triangle
                    if u >= 0. && v >= 0. && (u + v) <= 1. {
                        frag.push(FPoint::new(j as f32 + min_x, i as f32 + min_y));
                        let x = (j as f32 + min_x) as u32;
                        let y = (i as f32 + min_y) as u32;
                        
                        let px_idx = (y * self.width as u32 + x) as usize;
                        let byte_idx = px_idx * bytes_per_pixel;

                        if tri_data.1 >= drawndepths[px_idx] || byte_idx + bytes_per_pixel > self.px_buf.len() {
                            continue;
                        }

                        if let Some(off) = channel_offsets[0] { self.px_buf[byte_idx + off] = clr.r; }
                        if let Some(off) = channel_offsets[1] { self.px_buf[byte_idx + off] = clr.g; }
                        if let Some(off) = channel_offsets[2] { self.px_buf[byte_idx + off] = clr.b; }
                        if let Some(off) = channel_offsets[3] { self.px_buf[byte_idx + off] = clr.a; }

                        drawndepths[px_idx] = tri_data.1;
                    }
                }
            }
        }
    }
}