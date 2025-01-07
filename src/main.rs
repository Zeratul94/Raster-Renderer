#![allow(unused_must_use)]
#![allow(non_snake_case)]
#![allow(dead_code)]

extern crate sdl2;
extern crate glam;
extern crate rand;

use std::collections::HashMap;
use std::io::BufRead;
use std::io::BufReader;
use sdl2::pixels::Color;
use sdl2::rect::Point;
use sdl2::keyboard::Keycode;
use glam::Mat4;
use glam::Vec3;

fn main() {
    let SCREEN_WIDTH = 800;
    let SCREEN_HEIGHT = 600;

    let moveSpeed = 1.;
    
    //Read the OBJ file's data
    let uwb = std::env::current_dir().unwrap();
    let p = uwb.to_str().unwrap();
    let fullPath = format!("{}{}", p, "\\resources\\Leviathan");
    let fullPathstr = fullPath.as_str();
    let mut verts = Vec::new();
    let mut tris = Vec::new();
    let mut mats = Vec::new();
    readGeometry(fullPathstr, &mut verts, &mut tris, &mut mats);
    let mut viewer = Camera::new(Vec3::new(0., 0., 0.), 5., SCREEN_WIDTH, SCREEN_HEIGHT, 1000., 5.);

    let sdl = sdl2::init().unwrap();
    let vsub = sdl.video().unwrap();
    let mut window = vsub.window("Game", SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32).resizable().build().unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    let mut eventPump = sdl.event_pump().unwrap();
    //let mut now: std::time::Instant;
    sdl.mouse().set_relative_mouse_mode(true);
    'gl: loop {
        for event in eventPump.poll_iter() {match event {
                sdl2::event::Event::Quit {..} | sdl2::event::Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'gl,
                sdl2::event::Event::MouseMotion { xrel, yrel, .. } => {viewer.transform.rotate(1, xrel as f32 * 1.);
                                                                                 viewer.transform.rotate(0, yrel as f32 * 1.);},
                sdl2::event::Event::KeyDown { keycode: Some(Keycode::W), .. } => viewer.transform.offset(viewer.transform.forward * moveSpeed),
                sdl2::event::Event::KeyDown { keycode: Some(Keycode::S), .. } => viewer.transform.offset(viewer.transform.forward * -moveSpeed),
                sdl2::event::Event::KeyDown { keycode: Some(Keycode::A), .. } => viewer.transform.offset(viewer.transform.right * -moveSpeed),
                sdl2::event::Event::KeyDown { keycode: Some(Keycode::D), .. } => viewer.transform.offset(viewer.transform.right * moveSpeed),
                _ => {},
            }}
        
        canvas.set_draw_color(Color::BLACK);
        canvas.clear();
        
        let mut screenTris = Vec::new();
        for i in 0..tris.len() {
            if viewer.isinFrustum(tris[i], &verts) {
                let (vertex, depth) = viewer.projectTri(tris[i], &verts);
                if (vertex[0].x >= 0 || vertex[1].x >= 0 || vertex[2].x >= 0)
                && (vertex[0].x <= SCREEN_WIDTH || vertex[1].x <= SCREEN_WIDTH || vertex[2].x <= SCREEN_WIDTH)
                && (vertex[0].y >= 0 || vertex[1].y >= 0 || vertex[2].y >= 0)
                && (vertex[0].y <= SCREEN_HEIGHT || vertex[1].y <= SCREEN_HEIGHT || vertex[2].y <= SCREEN_HEIGHT) {
                    screenTris.push((vertex, depth, mats[i].color));
                }
            }
        }

        let l = screenTris.len();
        screenTris.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());//sort_by_key(|f| f.1);
        for i in 0..l {
            if canvas.draw_color() != screenTris[l-i-1].2 { canvas.set_draw_color(screenTris[l-i-1].2) }
            let frag = getPointsInTriangle(screenTris[l-i-1].0);
            canvas.draw_points(frag.as_slice());
        }

    //PRESENT and FPS TIMESTEP
        canvas.present();
        std::thread::sleep(std::time::Duration::new(0, 1_000_000_000u32 / 144 as u32));
    }
}

/* Structs */

#[derive(Clone, Copy)]
struct Point3 {
    x: f32,
    y: f32,
    z: f32
}

struct Plane {
    normal: Vec3,
    samplepoint: Vec3
}

struct Camera {
    transform: TransformComponent,
    F: f32,
    proj_mat: Mat4,
    target_width: i32,
    target_height: i32,
    aspect_ratio: f32,
    pixelscale: f32,

    frustumPlanes: [Plane; 6]
}

struct TransformComponent {
    transform: Mat4,
    invtransform: Mat4,
    location: Vec3,
    rotation: Mat4,
    scale: Vec3,

    forward: Vec3,
    right: Vec3,
}


#[derive(Clone, Copy)]
struct Material {
    color: Color
}

/* Implementations */

impl std::fmt::Display for Plane {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        return write!(f, "Normal {}, Passes Through {}", self.normal, self.samplepoint);
    }
}

impl Camera {
    pub fn new(location: Vec3, F: f32, target_width: i32, target_height: i32, drawdist: f32, clipdist:f32) -> Self {
        let pmat = Mat4::from_cols_array_2d(&[[F, 0., 0., 0.], [0., F, 0., 0.], [0., 0., 1., F], [0., 0., 0., 0.]]);
        let aspect = target_width as f32 / target_height as f32;

        let hh = target_height as f32 / 75.;
        //println!("{}", hh);
        let hw = hh * aspect;
        
        let nw = Vec3::new(-hw, hh, 0.);
        let ne = Vec3::new(hw, hh, 0.);
        let se = Vec3::new(hw, -hh, 0.);
        let sw = Vec3::new(-hw, -hh, 0.);

        let nw_dir = Vec3::new(-hw, hh, F);
        let ne_dir = Vec3::new(hw, hh, F);
        let se_dir = Vec3::new(hw, -hh, F);
        let sw_dir = Vec3::new(-hw, -hh, F);

        let frustumPlanes = [Plane { normal: -(nw_dir.cross(ne_dir)).normalize(), samplepoint: nw },
                                         Plane { normal: -(ne_dir.cross(se_dir)).normalize(), samplepoint: ne },
                                         Plane { normal: -(se_dir.cross(sw_dir)).normalize(), samplepoint: se },
                                         Plane { normal: -(sw_dir.cross(nw_dir)).normalize(), samplepoint: sw },
                                         Plane { normal: Vec3::new(0., 0., 1.), samplepoint: Vec3::new(0., 0., clipdist) },
                                         Plane { normal: Vec3::new(0., 0., -1.), samplepoint: Vec3::new(0., 0., drawdist) }];
        //println!("{},\n{},\n{},\n{},\n{},\n{}", frustumPlanes[0], frustumPlanes[1], frustumPlanes[2], frustumPlanes[3], frustumPlanes[4], frustumPlanes[5]);
        Self {transform: TransformComponent::new(location), F: F, proj_mat: pmat, target_width: target_width, target_height: target_height, aspect_ratio: aspect, pixelscale: 75., frustumPlanes: frustumPlanes}
    }

    pub fn isinFrustum(&self, tri: [usize; 3], verts: &Vec<Vec3>) -> bool {
        let mut vertsInside = [true; 3];
        for i in 0..3 {
            let v = verts[tri[i]];
            'checkvertex: for j in 0..6 {
                if (v - self.frustumPlanes[j].samplepoint).dot(self.frustumPlanes[j].normal) <= 0. {
                    vertsInside[i] = false;
                    break 'checkvertex;
                }
            }
        }
        vertsInside.contains(&true)
    }

    pub fn projectTri(&self, tri: [usize; 3], verts: &Vec<Vec3>) -> ([Point; 3], f32) {
        // For each vertex, calculate the corresponding screen location
        let p1 = self.projectPoint(self.transform.invtransform.project_point3(verts[tri[0]]));
        let p2 = self.projectPoint(self.transform.invtransform.project_point3(verts[tri[1]]));
        let p3 = self.projectPoint(self.transform.invtransform.project_point3(verts[tri[2]]));

        // Place the three screenspace points in an array and return them
        let frag = [p1, p2, p3];
        return (frag, (verts[tri[0]].z + verts[tri[1]].z + verts[tri[2]].z)/3.);
    }
    
    fn projectPoint(&self, v: Vec3) -> Point {
        let projv = self.proj_mat.project_point3(v);
        let dx = self.aspect_ratio*self.pixelscale*projv.x/projv.z;
        let dy = -self.pixelscale*projv.y/projv.z;
        return Point::new((self.target_width as f32/2. + dx) as i32, (self.target_height as f32/2. + dy) as i32);
    }
}

impl TransformComponent {
    pub fn new(location: Vec3) -> Self {
        let mut tmat = Mat4::IDENTITY;
        tmat.w_axis = location.extend(1.);
        let rotmat = Mat4::IDENTITY;
        Self {transform: tmat, invtransform: tmat.inverse(), location: location, rotation: rotmat, scale: Vec3::new(1., 1., 1.), forward: Vec3::new(0.,0.,0.), right: Vec3::new(0.,0.,0.)}
    }

    pub fn offset(&mut self, delta: Vec3) {
        let mut dmat = Mat4::IDENTITY;
        dmat.w_axis = delta.extend(1.);
        //println!("{}", self.location);
        self.location = dmat.project_point3(self.location);
        self.updateTransform();
        //println!("{}", self.location);
    }

    pub fn rotate(&mut self, /* 0 is X, 1 is Y, 2 is Z */ localAxis: i8, delta_degs: f32) {
        let drads = f32::to_radians(delta_degs);
        //println!("Angle: {} radians\nOld Rotation: {}", drads, self.rotation);
        // let cosine = f32::cos(drads);
        // let sine = f32::sin(drads);
        
        self.rotation.x_axis = self.rotation.x_axis.normalize();
        self.rotation.y_axis = self.rotation.y_axis.normalize();
        self.rotation.z_axis = self.rotation.z_axis.normalize();
        match localAxis {
            0 => {
                self.rotation.y_axis.y = f32::cos(f32::acos(self.rotation.y_axis.y) + drads);
                self.rotation.z_axis.y = -f32::sin(f32::asin(-self.rotation.z_axis.y) + drads);
                self.rotation.y_axis.z = f32::sin(f32::asin(self.rotation.y_axis.z) + drads);
                self.rotation.z_axis.z = f32::cos(f32::acos(self.rotation.z_axis.z) + drads);
            },
            1 => {
                self.rotation.x_axis.x = f32::cos(f32::acos(self.rotation.x_axis.x) + drads);
                self.rotation.z_axis.x = f32::sin(f32::asin(self.rotation.z_axis.x) + drads);
                self.rotation.x_axis.z = -f32::sin(f32::asin(-self.rotation.x_axis.z) + drads);
                self.rotation.z_axis.z = f32::cos(f32::acos(self.rotation.z_axis.z) + drads);
            },
            2 => {
                self.rotation.x_axis.x = f32::cos(f32::acos(self.rotation.x_axis.x) + drads);
                self.rotation.y_axis.x = -f32::sin(f32::asin(-self.rotation.y_axis.x) + drads);
                self.rotation.x_axis.y = f32::sin(f32::asin(self.rotation.x_axis.y) + drads);
                self.rotation.y_axis.y = f32::cos(f32::acos(self.rotation.y_axis.y) + drads);
            },
            _ => panic!("Invalid local axis: please use 0 for x, 1 for y, or 2 for z")
        }

        self.updateTransform();

        //println!("New Rotation: {}\n", self.rotation)
    }

    pub fn updateTransform(&mut self) {
        self.transform = self.offsetmatrix().mul_mat4(
                                                        &self.rotation.mul_mat4(
                                                            &self.scalematrix().mul_mat4(
                                                                &Mat4::IDENTITY
                                                            )
                                                        )
                                                    );
        self.invtransform = self.transform.inverse();

        self.forward = self.rotation.project_point3(Vec3 {x: 0., y: 0., z: 1.});
        self.right = self.rotation.project_point3(Vec3 {x: 1., y: 0., z: 0.});
    }
    
    pub fn scalematrix(&self) -> Mat4 {
        let mut m = Mat4::IDENTITY;
        m.x_axis.x = self.scale.x;
        m.y_axis.y = self.scale.y;
        m.z_axis.z = self.scale.z;
        m
    }

    pub fn offsetmatrix(&self) -> Mat4 {
        let mut m = Mat4::IDENTITY;
        m.w_axis = self.location.extend(1.);
        m
    }
}

impl Material {
    fn new(col: Color) -> Material {
        Material { color: col }
    }

    pub const DEFAULT: Self = Self { color: Color { r: 128, g: 128, b: 128, a: 255 } };
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
    if maxY > minY && maxX > minX {
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
    }
    return frag;
}

/*
fn rotate(dir: f32, vertices: &mut Vec<Vec3>, origin: Vec3) {
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
*/

fn readGeometry(p: &str, vertTarget: &mut Vec<Vec3>, triTarget: &mut Vec<[usize; 3]>, matTarget: &mut Vec<Material>) {
    println!("reading...");

    //Materials
    let rFile_mats: std::fs::File = std::fs::File::open(format!("{}{}", p, ".mtl")).expect("mtl file not found!");
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
    let rFile_geo = std::fs::File::open(format!("{}{}", p, ".obj")).expect("obj file not found!");
    let f_geo = BufReader::new(rFile_geo);
    for line in f_geo.lines().filter_map(|result| result.ok()) {
        if !line.is_empty() {
            let chars: Vec<char> = line.chars().collect();
            //Verts
            if chars[0] == 'v' && chars[1] == ' ' {
                let segs: Vec<&str> = line.split(' ').collect();
                vertTarget.push(Vec3 { x: segs[1].parse().unwrap(), y: segs[2].parse().unwrap(), z: segs[3].parse().unwrap() });
            }
            //Tris (and assign Mats)
            else if chars[0] == 'f' && chars[1] == ' ' {
                let segs: Vec<&str> = line.split(' ').collect();
                let is1: Vec<&str> = segs[1].split('/').collect();
                let is2: Vec<&str> = segs[2].split('/').collect();
                let is3: Vec<&str> = segs[3].split('/').collect();
                triTarget.push([is1[0].parse::<usize>().unwrap() - 1, is2[0].parse::<usize>().unwrap() - 1, is3[0].parse::<usize>().unwrap() - 1]);
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