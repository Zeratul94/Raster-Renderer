#![allow(unused_must_use)]
#![allow(dead_code)]

extern crate sdl3;
extern crate glam;

use derive_new::new;

use glam::{Vec3, Vec4};
use sdl3::pixels::PixelFormat;
use sdl3::render::FPoint;
use sdl3::video::WindowContext;

use crate::VertexData;

/* Structs */
pub type ScreenTri = ([FPoint; 3], f32, usize, [VertexData; 3]);

#[derive(Clone, Copy, new)]
pub struct Material {
    pub base_color: Vec4
}

#[derive(new)]
pub struct ShaderContext {
    light_pos: Vec3,
    light_dir: Vec3,
    light_color: sdl3::pixels::Color
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
    pub const DEFAULT: Self = Self { base_color: Vec4::new(0.5, 0.5, 0.5, 1.0) };
    fn frag(&self, u: f32, v: f32, geo_data: &VertexData, ctx: &ShaderContext) -> sdl3::pixels::Color {

        let clr = self.base_color;/*Vec4::new(
            u, // RED
            v, // GREEN
            geo_data.normal.z, // BLUE
            1.); // ALPHA*/
        sdl3::pixels::Color {
            r: (clr.x * 255.) as u8,
            g: (clr.y * 255.) as u8,
            b: (clr.z * 255.) as u8,
            a: (clr.w * 255.) as u8
        }
    }
}
impl std::fmt::Display for Material {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        return write!(f, "{}, {}, {}", self.base_color.x, self.base_color.y, self.base_color.z);
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
    pub fn render_tris(&mut self, tris: &mut Vec<([FPoint; 3], f32, usize, [VertexData; 3])>, materials: &Vec<Material>, ctx: &ShaderContext) {
        // Rasterize the projected triangles, and bake them to a texture
        let mut drawndepths = vec![f32::INFINITY; self.width as usize * self.height as usize];
        self.px_buf.fill(0);
        let l = tris.len();

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
            self.draw_pixels_in_triangle((tris[i].0, tris[i].3.each_ref()), &mut drawndepths, bytes_per_pixel, [r_offset, g_offset, b_offset, a_offset], &materials[tris[i].2], ctx);
        }

        // Write to the texture
        self.render_tex.update(None, &self.px_buf, self.width as usize * self.pixel_format.bytes_per_pixel() as usize).unwrap();
    }

    fn draw_pixels_in_triangle(&mut self, tri_data: ([FPoint; 3], [&VertexData; 3]), drawndepths: &mut Vec<f32>, bytes_per_pixel: usize, channel_offsets: [Option<usize>; 4], mat: &Material, ctx: &ShaderContext) {
        let tri = tri_data.0;
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
        if end_y <= start_y || end_x <= start_x { return; }

        /* Gemini block */
        // 1. Calculate the denominator (the area of the triangle)
        let area = (tri[1].y - tri[2].y) * (tri[0].x - tri[2].x) + (tri[2].x - tri[1].x) * (tri[0].y - tri[2].y);
        if area.abs() < 1e-6 { return; } // Avoid division by zero for degenerate triangles
        let inv_denom = 1.0 / area;

        // 2. The gradients (how much u and v change per pixel)
        let du_dx = (tri[1].y - tri[2].y) * inv_denom;
        let du_dy = (tri[2].x - tri[1].x) * inv_denom;
        let dv_dx = (tri[2].y - tri[0].y) * inv_denom;
        let dv_dy = (tri[0].x - tri[2].x) * inv_denom;

        // 3. The initial U and V for the very first pixel at (start_x, start_y)
        // We calculate these by evaluating the barycentric formula at the specific screen point
        let dx = start_x as f32 - tri[2].x;
        let dy = start_y as f32 - tri[2].y;
        let start_u = ((tri[1].y - tri[2].y) * dx + (tri[2].x - tri[1].x) * dy) * inv_denom;
        let start_v = ((tri[2].y - tri[0].y) * dx + (tri[0].x - tri[2].x) * dy) * inv_denom;
        
        let w0 = 1./tri_data.1[0].depth;
        let w1 = 1./tri_data.1[1].depth;
        let w2 = 1./tri_data.1[2].depth;

        let mut row_u = start_u;
        let mut row_v = start_v;
        /* end of Gemini block */
        for y in start_y..end_y {
            let mut u = row_u;
            let mut v = row_v;
            let row_idx = y as usize * self.width as usize;
            for x in start_x..end_x {
                let w = 1. - u - v;
                let pixel_depth = 1. / ((w0 * u) + (w1 * v) + (w2 * w));

                // If u and v are both between 0 and 1, the point is inside the triangle
                if u >= -0.0001 && v >= -0.0001 && (u + v) <= 1.0001 {
                    let px_idx = row_idx + x as usize;
                    let byte_idx = px_idx * bytes_per_pixel;

                    if pixel_depth >= drawndepths[px_idx] || byte_idx + bytes_per_pixel > self.px_buf.len() {
                        continue;
                    }

                    let vert_data = VertexData {
                        normal: (tri_data.1[0].normal * u + tri_data.1[1].normal * v + tri_data.1[2].normal * w).normalize(),
                        position: tri_data.1[0].position * u + tri_data.1[1].position * v + tri_data.1[2].position * w,
                        depth: pixel_depth
                    };

                    let clr = mat.frag(u, v, &vert_data, ctx);
                    if let Some(off) = channel_offsets[0] { self.px_buf[byte_idx + off] = clr.r; }
                    if let Some(off) = channel_offsets[1] { self.px_buf[byte_idx + off] = clr.g; }
                    if let Some(off) = channel_offsets[2] { self.px_buf[byte_idx + off] = clr.b; }
                    if let Some(off) = channel_offsets[3] { self.px_buf[byte_idx + off] = clr.a; }

                    drawndepths[px_idx] = pixel_depth;
                }
                u += du_dx;
                v += dv_dx;
            }
            row_u += du_dy;
            row_v += dv_dy;
        }
    }
}