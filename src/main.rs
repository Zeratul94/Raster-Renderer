#![allow(unused_must_use)]
#![allow(dead_code)]

extern crate sdl3;
extern crate glam;
extern crate rand;

mod engine;

pub use engine::*;

use sdl3::pixels::Color;
use sdl3::render::FPoint;
use sdl3::keyboard::Keycode;

use glam::Vec3;

fn main() {
    static SCREEN_WIDTH: u32 = 1280;
    static SCREEN_HEIGHT: u32 = 720;
    static FRUSTUM_INSET: f32 = 5.;

    let move_speed = 1.;
    
    //Read the OBJ file's data
    let uwb = std::env::current_dir().unwrap();
    let p = uwb.to_str().unwrap();
    let full_path = format!("{}{}", p, "\\resources\\Leviathan");
    let full_path_str = full_path.as_str();
    let mut verts = Vec::new();
    let mut tris = Vec::new();
    let mut mats = Vec::new();
    read_geometry(full_path_str, &mut verts, &mut tris, &mut mats);

    let mut viewer = Camera::new(Vec3::new(0., 0., 0.), 2.5, SCREEN_WIDTH, SCREEN_HEIGHT, 1000., FRUSTUM_INSET/* clipdist */, FRUSTUM_INSET);

    let sdl = sdl3::init().unwrap();
    let vsub = sdl.video().unwrap();
    let window = vsub.window("Game", SCREEN_WIDTH, SCREEN_HEIGHT).resizable().build().unwrap();
    
    sdl.mouse().set_relative_mouse_mode(&window, true);
    sdl.mouse();

    let mut canvas = window.into_canvas();
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
        
        canvas.set_draw_color(Color::BLACK);
        canvas.clear();
        
        let mut screen_tris = Vec::new();
        for i in 0..tris.len() {
            match viewer.clip_tri_to_frustum(tris[i], &verts) {
                Some(clipped_tri) => { // If the triangle is valid, render it
                    for tri in clipped_tri.iter() {
                        let (screen_tri, depth) = viewer.project_tri(*tri);

                        if (screen_tri[0].x >= 0. || screen_tri[1].x >= 0. || screen_tri[2].x >= 0.) // If any of the projected vertices are actually on-screen (should be redundant
                        && (screen_tri[0].x <= SCREEN_WIDTH as f32 || screen_tri[1].x <= SCREEN_WIDTH as f32 || screen_tri[2].x <= SCREEN_WIDTH as f32) // with frustum culling)
                        && (screen_tri[0].y >= 0. || screen_tri[1].y >= 0. || screen_tri[2].y >= 0.)
                        && (screen_tri[0].y <= SCREEN_HEIGHT as f32 || screen_tri[1].y <= SCREEN_HEIGHT as f32 || screen_tri[2].y <= SCREEN_HEIGHT as f32) {
                            screen_tris.push((screen_tri, depth, mats[i].color));
                        }
                    }
                },
                None => {}, // If the triangle is invalid, do nothing
            }
        }

        let l = screen_tris.len();
        screen_tris.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());//sort_by_key(|f| f.1);
        for i in 0..l {
            if canvas.draw_color() != screen_tris[l-i-1].2 { canvas.set_draw_color(screen_tris[l-i-1].2) }
            let frag: Vec<FPoint> = get_points_in_triangle(screen_tris[l-i-1].0);
            canvas.draw_points(frag.as_slice());
        }

    //PRESENT and FPS TIMESTEP
        canvas.present();
        std::thread::sleep(std::time::Duration::new(0, 1_000_000_000u32 / 144 as u32));
    }
    
}