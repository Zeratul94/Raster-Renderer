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
        let mut drawnpx: Vec<FPoint> = Vec::new();
        self.px_buf.fill(0);
        let l = tris.len();
        tris.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap().reverse()); // We draw the triangles from back to front because we haven't yet implemented overdraw protection

        for i in 0..l {
            println!("chkpt 1");
            // Collect the pixels in the triangle which have not already been drawn this frame
            let clr = materials[tris[i].2].color;
            println!("chkpt 2");
            let mut frag: Vec<FPoint> = Self::get_points_in_triangle(tris[i].0);
            println!("chkpt 3");
            //frag.retain(|x| !(drawnpx.contains(x))); // Why the heck doesn't this work?
            println!("chkpt 4");
            // Draw the collected pixels to the pixel buffer
            for p in frag.iter() {
                let x = p.x as u32;
                let y = p.y as u32;
                if !p.x.is_finite() || !p.y.is_finite()
                 || p.x < 0. || p.y < 0.
                 || x > self.width as u32 - 1 || y > self.height as u32 - 1 { continue; }
                
                let bytes_per_pixel = self.pixel_format.bytes_per_pixel() as usize;
                let idx = ((y * self.width as u32 + x) * bytes_per_pixel as u32) as usize;
                if idx + bytes_per_pixel > self.px_buf.len() {continue;}
                
                let masks = self.pixel_format.into_masks().unwrap();
                
                for subidx in 0..self.pixel_format.bytes_per_pixel() as usize {
                    if (masks.rmask >> (subidx * 8) & 0xFF) != 0 {
                        self.px_buf[idx + subidx] = clr.r;
                    } else if (masks.gmask >> (subidx * 8) & 0xFF) != 0 {
                        self.px_buf[idx + subidx] = clr.g;
                    } else if (masks.bmask >> (subidx * 8) & 0xFF) != 0 {
                        self.px_buf[idx + subidx] = clr.b;
                    } else if (masks.amask >> (subidx * 8) & 0xFF) != 0 {
                        self.px_buf[idx + subidx] = clr.a;
                    } else {
                        self.px_buf[idx + subidx] = 0;
                    }
                }
            }
            drawnpx.extend(frag.iter());
        }

        // Write to the texture
        self.render_tex.update(None, &self.px_buf, self.width as usize * self.pixel_format.bytes_per_pixel() as usize).unwrap();
    }

}

// Static methods for Surface
impl Surface<'_> {
    // Static method to convert a triangle on the screen and get the
    // pixels to draw to render it
    fn get_points_in_triangle(tri: [FPoint; 3]) -> Vec<FPoint> {
        let min_x = f32::min(f32::min(tri[0].x, tri[1].x), tri[2].x);
        let max_x = f32::max(f32::max(tri[0].x, tri[1].x), tri[2].x);
        let min_y = f32::min(f32::min(tri[0].y, tri[1].y), tri[2].y);
        let max_y = f32::max(f32::max(tri[0].y, tri[1].y), tri[2].y);

        let mut frag = Vec::new();
        if max_y > min_y && max_x > min_x {
            //let mut printed_yet = false; //DEBUG
            for i in 0..=(max_y-min_y).trunc() as i32 {
                for j in 0..=(max_x-min_x).trunc() as i32 {
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
                    }
                }
            }
        }
        return frag;
    }
}