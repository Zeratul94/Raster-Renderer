#![allow(unused_must_use)]
#![allow(dead_code)]
#![allow(non_snake_case)]

extern crate sdl3;
extern crate glam;
extern crate rand;

mod geo_engine;
mod gfx_engine;

use geo_engine::*;
use gfx_engine::ScreenTri;

use sdl3::pixels::{Color, PixelFormat};
use sdl3::render::{FPoint, Texture, TextureAccess, Vertex};
use sdl3::keyboard::Keycode;
use sdl3::event::Event::KeyDown;

use glam::Vec3;

#[derive(Clone, Copy)]
pub struct PreRenderData {
    SCREEN_WIDTH: u16,
    SCREEN_HEIGHT: u16,
    FRUSTUM_INSET: f32,
    CLIPDIST: f32,
}

const MOVE_SPEED: f32 = 1.;

pub fn start(title: &str, pre_data: PreRenderData, geometry: &Vec<Mesh>, materials: &Vec<gfx_engine::Material>) {
    let sdl = sdl3::init().unwrap();
    let vsub = sdl.video().unwrap();
    let window = vsub.window("Raster Renderer", pre_data.SCREEN_WIDTH as u32, pre_data.SCREEN_HEIGHT as u32).resizable().build().unwrap();
    let mut canvas = window.into_canvas();
    let texture_creator = canvas.texture_creator();
    let pixel_format = canvas.default_pixel_format();
    let context = gfx_engine::ShaderContext::new(Vec3::ZERO, Vec3::ZERO, sdl3::pixels::Color::WHITE);

    let mut render_surf = gfx_engine::Surface::new(pre_data.SCREEN_WIDTH, pre_data.SCREEN_HEIGHT, pixel_format, &texture_creator);
    let mut viewer = Camera::new(Vec3::new(20., 0., -20.), 2.5, pre_data.SCREEN_WIDTH as u32, pre_data.SCREEN_HEIGHT as u32,
                                         1000., pre_data.CLIPDIST, pre_data.FRUSTUM_INSET);
    
    let mut first_frame = true;
    let mut event_pump = sdl.event_pump().unwrap();
    'gl: loop {
        if first_frame {
            sdl.mouse().set_relative_mouse_mode(canvas.window(), true);
            sdl.mouse().warp_mouse_in_window(canvas.window(), (pre_data.SCREEN_WIDTH/2) as f32, (pre_data.SCREEN_HEIGHT/2) as f32);
            sdl.mouse().show_cursor(false);
            first_frame = false;
        }
        let _start_frametime = std::time::Instant::now();
        for event in event_pump.poll_iter() {match event {
                sdl3::event::Event::Quit {..} | sdl3::event::Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'gl,
                sdl3::event::Event::MouseMotion { xrel, yrel, .. } => {viewer.transform.rotate(1, xrel as f32 * 1.);
                                                                                 viewer.transform.rotate(0, yrel as f32 * -1.);
                                                                                 viewer.update_frustum_planes();},
                KeyDown { keycode: Some(Keycode::Right), .. } => {viewer.transform.rotate(1, 5.); viewer.update_frustum_planes();},
                KeyDown { keycode: Some(Keycode::W), .. } => {viewer.transform.offset(viewer.transform.forward * MOVE_SPEED); viewer.update_frustum_planes();},
                KeyDown { keycode: Some(Keycode::S), .. } => {viewer.transform.offset(viewer.transform.forward * -MOVE_SPEED); viewer.update_frustum_planes();},
                KeyDown { keycode: Some(Keycode::A), .. } => {viewer.transform.offset(viewer.transform.right * -MOVE_SPEED); viewer.update_frustum_planes();},
                KeyDown { keycode: Some(Keycode::D), .. } => {viewer.transform.offset(viewer.transform.right * MOVE_SPEED); viewer.update_frustum_planes();},
                _ => {},
            }}
        
        //let mousex; let mousey;
        //sdl3-sys::SDL_GetMouseState(&mut mousex, &mut mousey);
        
        let mut screen_tris = geometry.iter()
                                                                    .flat_map(|mesh| viewer.project_mesh(mesh))
                                                                    .collect::<Vec<ScreenTri>>();

        // Draw the projected geometry to the render surface
        render_surf.render_tris(&mut screen_tris, &materials, &context);

    // Present the scene and FPS Timestep
        canvas.set_draw_color(Color::BLACK);
        canvas.clear();
        canvas.copy(&render_surf.render_tex, None, None);
        canvas.present();

    }
    
}

fn main() {
    static PRE_DATA: PreRenderData = PreRenderData {
        SCREEN_WIDTH: 1280,
        SCREEN_HEIGHT: 720,
        FRUSTUM_INSET: -1.,
        CLIPDIST: 1.
    };
    
    //Read the OBJ file's data
    let uwb = std::env::current_dir().unwrap();
    let path_prefix = uwb.to_str().unwrap();
    let local_path = "/resources/Leviathan";

    let mut mat_lib = MaterialLibrary::new();
    let geo = vec![Mesh::from_obj(path_prefix, local_path, Vec3::new(0., 0., 0.), &mut mat_lib)];
    start("Raster Renderer", PRE_DATA, &geo, &mat_lib.materials);
}