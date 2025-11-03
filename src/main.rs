#![allow(unused_must_use)]
#![allow(dead_code)]
#![allow(non_snake_case)]

extern crate sdl3;
extern crate glam;
extern crate rand;

mod geo_engine;
mod gfx_engine;

pub use geo_engine::*;

use sdl3::pixels::{Color, PixelFormat};
use sdl3::render::{FPoint, Texture, TextureAccess};
use sdl3::keyboard::Keycode;

use glam::Vec3;


fn main() {
    static SCREEN_WIDTH: u16 = 1280;
    static SCREEN_HEIGHT: u16 = 720;
    static FRUSTUM_INSET: f32 = 5.;

    let move_speed = 1.;
    
    //Read the OBJ file's data
    let uwb = std::env::current_dir().unwrap();
    let path_prefix = uwb.to_str().unwrap();
    let local_path = "/resources/Leviathan";
    let mut verts = Vec::new();
    let mut tris = Vec::new();
    let mut matIdcs = Vec::new();
    let mut materials = Vec::new();
    read_geometry(path_prefix, local_path, &mut verts, &mut tris, &mut matIdcs, &mut materials);

    let sdl = sdl3::init().unwrap();
    let vsub = sdl.video().unwrap();
    let window = vsub.window("Raster Renderer", SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32).resizable().build().unwrap();
    
    sdl.mouse();
    sdl.mouse().set_relative_mouse_mode(&window, true);
    sdl.mouse().warp_mouse_in_window(&window, (SCREEN_WIDTH/2) as f32, (SCREEN_HEIGHT/2) as f32);

    let mut canvas = window.into_canvas();

    let texture_creator = canvas.texture_creator();
    let pixel_format = canvas.default_pixel_format();
    let mut render_surf = gfx_engine::Surface::new(SCREEN_WIDTH, SCREEN_HEIGHT, pixel_format, &texture_creator);
    let mut viewer = Camera::new(Vec3::new(0., 0., 0.), 2.5, SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32, 1000., FRUSTUM_INSET/* clipdist */, FRUSTUM_INSET);
    
    let mut event_pump = sdl.event_pump().unwrap();

    //let mut now: std::time::Instant;
    'gl: loop {
        for event in event_pump.poll_iter() {match event {
                sdl3::event::Event::Quit {..} | sdl3::event::Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'gl,
                sdl3::event::Event::MouseMotion { xrel, yrel, .. } => {viewer.transform.rotate(1, xrel as f32 * 1.);
                                                                                 viewer.transform.rotate(0, yrel as f32 * -1.);},
                sdl3::event::Event::KeyDown { keycode: Some(Keycode::Right), .. } => viewer.transform.rotate(1, 5.),
                sdl3::event::Event::KeyDown { keycode: Some(Keycode::W), .. } => viewer.transform.offset(viewer.transform.forward * move_speed),
                sdl3::event::Event::KeyDown { keycode: Some(Keycode::S), .. } => viewer.transform.offset(viewer.transform.forward * -move_speed),
                sdl3::event::Event::KeyDown { keycode: Some(Keycode::A), .. } => viewer.transform.offset(viewer.transform.right * -move_speed),
                sdl3::event::Event::KeyDown { keycode: Some(Keycode::D), .. } => viewer.transform.offset(viewer.transform.right * move_speed),
                _ => {},
            }}
        
        //let mousex; let mousey;
        //sdl3-sys::SDL_GetMouseState(&mut mousex, &mut mousey);
        
        // Project geometry to the screen
        let mut screen_tris = Vec::new();
        for i in 0..tris.len() {
            match viewer.clip_tri_to_frustum(tris[i], &verts) {
                Some(clipped_tri) => { // If the triangle is valid, render it
                    for tri in clipped_tri.iter() {
                        let (screen_tri, depth) = viewer.project_tri(*tri);
                        if render_surf.clip_tri_to_screen(screen_tri) {
                            screen_tris.push((screen_tri, depth, matIdcs[i]));
                        }
                    }
                },
                None => {}, // If the triangle is invalid, do nothing
            }
        }

        render_surf.render_tris(&mut screen_tris, &materials);

    // Present the scene and FPS Timestep
        canvas.set_draw_color(Color::BLACK);
        canvas.clear();
        canvas.copy(&render_surf.render_tex, None, None);
        canvas.present();
        std::thread::sleep(std::time::Duration::new(0, 1_000_000_000u32 / 144 as u32));
    }
    
}