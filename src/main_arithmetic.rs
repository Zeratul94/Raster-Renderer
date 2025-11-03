#![allow(unused_must_use)]
#![allow(non_snake_case)]
#![allow(dead_code)]

extern crate sdl2;
extern crate rand;

use std::collections::HashMap;
use std::io::BufRead;
use std::io::BufReader;
use sdl2::pixels::Color;
use sdl2::rect::Point;
use sdl2::keyboard::Keycode;


fn main() {
    let SCREEN_WIDTH = 800;
    let SCREEN_HEIGHT = 600;
    
    //Read the OBJ file's data
    let uwb = std::env::current_dir().unwrap();
    let p = uwb.to_str().unwrap();
    let fullP = format!("{}{}", p, "\\resources\\Leviathan");
    let fullPstr = fullP.as_str();
    let mut verts = Vec::new();
    let mut tris = Vec::new();
    let mut mats = Vec::new();
    readGeometry(fullPstr, &mut verts, &mut tris, &mut mats);
    let /* mut */ viewer = Camera {rotation: Point3 { x: 0., y: 0., z: 0. }, F: 5., target_width: SCREEN_WIDTH, target_height: SCREEN_HEIGHT};

    let sdl = sdl2::init().unwrap();
    let vsub = sdl.video().unwrap();
    let window = vsub.window("Game", SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32).resizable().build().unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    let mut eventPump = sdl.event_pump().unwrap();
    let vertmutref = &mut verts;
    // let mut now: std::time::Instant;
    'gl: loop {
        for event in eventPump.poll_iter() {match event {
                sdl2::event::Event::Quit {..} | sdl2::event::Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'gl,
                sdl2::event::Event::KeyDown { keycode: Some(Keycode::Right), .. } => rotate(1., vertmutref, vertmutref[vertmutref.len()-1]),
                sdl2::event::Event::KeyDown { keycode: Some(Keycode::Left), .. } => rotate(-1., vertmutref, vertmutref[vertmutref.len()-1]),
                sdl2::event::Event::KeyDown { keycode: Some(Keycode::Up), .. } => zoom(-0.25, vertmutref),
                sdl2::event::Event::KeyDown { keycode: Some(Keycode::Down), .. } => zoom(0.25, vertmutref),
                _ => {},
            }}
        canvas.set_draw_color(Color::BLACK);
        canvas.clear();
        
        let mut screenTris = Vec::new();
        for i in 0..tris.len() {
            let (vertex, depth) = viewer.projectWorldToScreen(tris[i], vertmutref);
            if (vertex[0].x >= 0 || vertex[1].x >= 0 || vertex[2].x >= 0)
            && (vertex[0].x <= SCREEN_WIDTH || vertex[1].x <= SCREEN_WIDTH || vertex[2].x <= SCREEN_WIDTH)
            && (vertex[0].y >= 0 || vertex[1].y >= 0 || vertex[2].y >= 0)
            && (vertex[0].y <= SCREEN_HEIGHT || vertex[1].y <= SCREEN_HEIGHT || vertex[2].y <= SCREEN_HEIGHT) {
                screenTris.push((vertex, (depth*53.) as i32, mats[i].color));
            }
        }
        let l = screenTris.len();
        screenTris.sort_by_key(|f| f.1);
        for i in 0..l {
            if canvas.draw_color() != screenTris[l-i-1].2 { canvas.set_draw_color(screenTris[l-i-1].2) }
            let frag = getPointsInTriangle(screenTris[l-i-1].0);
            canvas.draw_points(frag.as_slice());
        }

    //PRESENT and FPS TIMESTEP
        canvas.present();
        std::thread::sleep(std::time::Duration::new(0, 1_000_000_000u32 / 144 as u32));
        // now = std::time::Instant::now();
        // println!("{}", now.elapsed().as_nanos());
    }
}

/* Structs */

#[derive(Clone, Copy)]
struct Point3 {
    x: f32,
    y: f32,
    z: f32
}

struct Camera {
    rotation: Point3,
    F: f32,
    target_width: i32,
    target_height: i32
}

#[derive(Clone, Copy)]
struct Material {
    color: Color
}

/* Implementations */

impl Camera {
    fn projectWorldToScreen(&self, tri: (usize, usize, usize), verts: &mut Vec<Point3>) -> ([Point; 3], f32) {
        
        let aspect_ratio = self.target_width as f32 / self.target_height as f32;
        let scale = 75.;
        // For each vertex, calculate the corresponding screen location
        let v1 = verts[tri.0];
        let dx = -f32::signum(v1.z)*aspect_ratio*scale*v1.x*self.F/(f32::abs(v1.z) + self.F);
        let dy = -f32::signum(v1.z)      *       scale*v1.y*self.F/(f32::abs(v1.z) + self.F);
        let p1 = Point::new((self.target_width as f32/2. + dx) as i32, (self.target_height as f32/2. + dy) as i32);
        
        let v2 = verts[tri.1];
        let dx = -f32::signum(v2.z)*aspect_ratio*scale*v2.x*self.F/(f32::abs(v2.z) + self.F);
        let dy = -f32::signum(v2.z)      *       scale*v2.y*self.F/(f32::abs(v2.z) + self.F);
        let p2 = Point::new((self.target_width as f32/2. + dx) as i32, (self.target_height as f32/2. + dy) as i32);
        
        let v3 = verts[tri.2];
        let dx = -f32::signum(v3.z)*aspect_ratio*scale*v3.x*self.F/(f32::abs(v3.z) + self.F);
        let dy = -f32::signum(v3.z)      *       scale*v3.y*self.F/(f32::abs(v3.z) + self.F);
        let p3 = Point::new((self.target_width as f32/2. + dx) as i32, (self.target_height as f32/2. + dy) as i32);
        
        // Place the three screenspace points in an array and return them
        let frag = [p1, p2, p3];
        return (frag, (v1.z + v2.z + v3.z)/3.);
    }
}

impl Material {
    fn new(col: Color) -> Material {
        Material { color: col }
    }

    pub const DEFAULT: Material = Material { color: Color { r: 128, g: 128, b: 128, a: 255 } };
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

/* Global Functions */

fn getPointsInTriangle(tri: [Point; 3]) -> Vec<Point> {
    let minX = i32::min(i32::min(tri[0].x, tri[1].x), tri[2].x);
    let maxX = i32::max(i32::max(tri[0].x, tri[1].x), tri[2].x);
    let minY = i32::min(i32::min(tri[0].y, tri[1].y), tri[2].y);
    let maxY = i32::max(i32::max(tri[0].y, tri[1].y), tri[2].y);

    let mut frag = Vec::new();
    for i in 0..=(maxY-minY) {
        for j in 0..=(maxX-minX) {
            let y1 = tri[0].y - minY;
            let y2 = tri[1].y - minY;
            let y3 = tri[2].y - minY;
            let x1 = tri[0].x - minX;
            let x2 = tri[1].x - minX;
            let x3 = tri[2].x - minX;
            let denom = (y2 - y3) * (x1 - x3) + (x3 - x2) * (y1 - y3);
            
            let w1;
            let w2;
            if denom != 0 {
                w1 = ((y2 - y3) * (j - x3) + (x3 - x2) * (i - y3)) as f32 / denom as f32;
                w2 = ((y3 - y1) * (j - x3) + (x1 - x3) * (i - y3)) as f32 / denom as f32;
            } else {
                w1 = ((y2 - y3) * (j - x3) + (x3 - x2) * (i - y3)) as f32;
                w2 = ((y3 - y1) * (j - x3) + (x1 - x3) * (i - y3)) as f32;
            }
            let w3 = 1. - w1 - w2;
            if w3 >= 0. && w2 >= 0. && w1 >= 0. {
                frag.push(Point::new(j + minX, i + minY))
            }
        }
    }
    return frag;
}

fn rotate(dir: f32, vertices: &mut Vec<Point3>, origin: Point3) {
    for vertex in vertices {
        vertex.x -= origin.x;
        vertex.z -= origin.z;
        let x = vertex.x;
        let z = vertex.z;
        vertex.x -= z*dir/6.28318531;
        vertex.z += x*dir/6.28318531;

        if vertex.x != 0. || vertex.z != 0. {
            let radScale = f32::sqrt(x*x + z*z)/f32::sqrt(vertex.x*vertex.x + vertex.z*vertex.z);
            vertex.x *= radScale;
            vertex.z *= radScale;
        }

        vertex.x += origin.x;
        vertex.z += origin.z;
    }
}

fn zoom(dir: f32, vertices: &mut Vec<Point3>) {
    for vertex in vertices {
        vertex.z += dir;
    }
}

fn readGeometry(p: &str, vertTarget: &mut Vec<Point3>, triTarget: &mut Vec<(usize, usize, usize)>, matTarget: &mut Vec<Material>) {
    println!("reading...");

    //Materials
    let rFile_mats: std::fs::File = std::fs::File::open(format!("{}{}", p, ".mtl")).expect("cannot discover file!");
    let f_mats = BufReader::new(rFile_mats);
    let mut materials = HashMap::new();
    let mut matName: String = String::new();
    let mut matCol = Material::DEFAULT.color;
    for line in f_mats.lines().filter_map(|result| result.ok()) {
        if !line.is_empty() {
            let segs: Vec<&str> = line.split(' ').collect();
            if segs[0] == "newmtl" {
                matName = segs[1].to_string().clone();
            }
            else if segs[0] == "Kd" {
                matCol = Color::RGB((segs[1].parse::<f32>().unwrap() * 255.) as u8, (segs[2].parse::<f32>().unwrap() * 255.) as u8, (segs[3].parse::<f32>().unwrap() * 255.) as u8)
            }

            materials.insert(matName.clone(), Material { color: matCol });
        }
    }

    //from now on matName is the name of the currently referenced material,
    //used to key to a certain material in the Map.

    //Geometry
    let rFile_geo = std::fs::File::open(format!("{}{}", p, ".obj")).expect("cannot discover file!");
    let f_geo = BufReader::new(rFile_geo);
    for line in f_geo.lines().filter_map(|result| result.ok()) {
        if !line.is_empty() {
            let chars: Vec<char> = line.chars().collect();
            //Verts
            if chars[0] == 'v' && chars[1] == ' ' {
                let segs: Vec<&str> = line.split(' ').collect();
                vertTarget.push(Point3 { x: segs[1].parse().unwrap(), y: segs[2].parse().unwrap(), z: segs[3].parse().unwrap() });
            }
            //Tris (and assign Mats)
            else if chars[0] == 'f' && chars[1] == ' ' {
                let segs: Vec<&str> = line.split(' ').collect();
                let is1: Vec<&str> = segs[1].split('/').collect();
                let is2: Vec<&str> = segs[2].split('/').collect();
                let is3: Vec<&str> = segs[3].split('/').collect();
                triTarget.push((is1[0].parse::<usize>().unwrap() - 1, is2[0].parse::<usize>().unwrap() - 1, is3[0].parse::<usize>().unwrap() - 1));
                matTarget.push(*materials.get(&matName).unwrap());
            }
            //Loading Mats
            else {
                let segs: Vec<&str> = line.split(' ').collect();
                if segs[0] == "usemtl" {
                    matName = segs[1].to_string();
                }
            }
        }
    }
    println!("done!");
}