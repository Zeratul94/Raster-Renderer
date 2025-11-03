#![allow(unused_must_use)]
#![allow(dead_code)]

extern crate sdl3;
extern crate glam;
extern crate rand;

use crate::gfx_engine;

use std::collections::HashMap;
use std::io::BufRead;
use std::io::BufReader;

use sdl3::pixels::Color;
use sdl3::render::FPoint;

use glam::Mat4;
use glam::Vec3;

use gfx_engine::Material;


/* Structs */

#[derive(Clone, Copy)]
pub struct Plane {
    pub normal: Vec3,
    pub samplepoint: Vec3
}

pub struct Camera {
    pub transform: TransformComponent,
    pub focal_length: f32,
    proj_mat: Mat4,
    pub target_width: u32,
    pub target_height: u32,
    pub aspect_ratio: f32,
    pixelscale: f32,

    frustum_planes: [Plane; 6]
}

pub struct TransformComponent {
    pub transform: Mat4,
    pub invtransform: Mat4,
    pub location: Vec3,
    /*pub rotation: Mat4,*/
    pub rotation: Vec3,
    pub scale: Vec3,

    pub forward: Vec3,
    pub right: Vec3,
}


/* Implementations */

impl Plane {
    pub fn intersect_line(&self, line_start: Vec3, line_end: Vec3) -> Vec3 {
        let line_slope = line_end - line_start;
        
        if self.normal.dot(line_slope).abs() < 0.1 {println!("slope denom: {}", self.normal.dot(line_slope));}
        let t = -self.normal.dot(line_start - self.samplepoint) / self.normal.dot(line_slope);
        line_slope*t + line_start
    }
}

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
    
    pub fn new(location: Vec3, focal_length: f32, target_width: u32, target_height: u32, draw_dist: f32, clip_dist: f32, frustum_inset: f32) -> Self {
        let pmat = Mat4::from_cols_array_2d(&[[focal_length, 0., 0., 0.], [0., focal_length, 0., 0.], [0., 0., 1., focal_length], [0., 0., 0., 0.]]);
        let aspect = target_width as f32 / target_height as f32;

        let hh = (target_height as f32 / 75.) - frustum_inset;
        //println!("{}", hh);
        let hw = (hh + frustum_inset) * aspect - frustum_inset;
        
        let nw = Vec3::new(-hw, -hh, 0.);
        let ne = Vec3::new(hw, -hh, 0.);
        let se = Vec3::new(hw, hh, 0.);
        let sw = Vec3::new(-hw, hh, 0.);

        let nw_dir = Vec3::new(-hw, -hh, focal_length);
        let ne_dir = Vec3::new(hw, -hh, focal_length);
        let se_dir = Vec3::new(hw, hh, focal_length);
        let sw_dir = Vec3::new(-hw, hh, focal_length);

        let frustum_planes = [Plane { normal: (nw_dir.cross(ne_dir)).normalize(), samplepoint: nw },
                                         Plane { normal: (ne_dir.cross(se_dir)).normalize(), samplepoint: ne },
                                         Plane { normal: (se_dir.cross(sw_dir)).normalize(), samplepoint: se },
                                         Plane { normal: (sw_dir.cross(nw_dir)).normalize(), samplepoint: sw },
                                         Plane { normal: Vec3::new(0., 0., 1.), samplepoint: Vec3::new(0., 0., clip_dist) },
                                         Plane { normal: Vec3::new(0., 0., -1.), samplepoint: Vec3::new(0., 0., draw_dist) }];
        //println!("{},\n{},\n{},\n{},\n{},\n{}", frustum_planes[0], frustum_planes[1], frustum_planes[2], frustum_planes[3], frustum_planes[4], frustum_planes[5]);
        Self {transform: TransformComponent::new(location), focal_length: focal_length, proj_mat: pmat, target_width: target_width, target_height: target_height, aspect_ratio: aspect, pixelscale: 75., frustum_planes: frustum_planes}
    }

    // Check if a triangle is wholly or partially in the view frustum, and return one of three options:
    // 1. If 0 vertices are visible, return None.
    // 2. If 1 or 3 vertices are contained, return Some(triangle) with the triangle or the version of it
    // clipped to the frustum.
    // 3. If 2 vertices are contained, return the two triangles that make up the rect that is visible.
    pub fn clip_tri_to_frustum(&self, tri: [usize; 3], verts: &Vec<Vec3>) -> Option<Vec<[Vec3; 3]>> {
        // Get the vertices of the triangle
        let v1 = verts[tri[0]];
        let v2 = verts[tri[1]];
        let v3 = verts[tri[2]];

        // Check if the triangle is in the frustum
        let mut visible_verts = [3, 3];
        let mut planes_not_passed = [vec![], vec![]];
        
        for i in 0..3 {
            let v = match i {
                0 => v1,
                1 => v2,
                _ => v3,
            };
            let mut planes_passed = 0;
            for plane in self.frustum_planes.iter() {
                if plane.normal.dot(v - plane.samplepoint) >= 0. {
                    planes_passed += 1;
                } else {
                    // Store the index of the plane which this is outside of
                    if visible_verts[0] == 3 {
                        planes_not_passed[0].push(i);
                    } else {
                        planes_not_passed[1].push(i);
                    }
                    // We still need to check the other planes (don't break), so we can find the
                    // closest intersection point in the clipping step
                }
            }
            if planes_passed >= 6 {
                // If all planes are passed, store the vertex index in the next available slot
                if visible_verts[0] != 3 {
                    visible_verts[0] = i;
                } else if visible_verts[1] != 3 {
                    visible_verts[1] = i;
                } else {
                    return Some(vec![[v1, v2, v3]]); // If both slots are filled and the third vertex is also visible, the triangle is fully visible
                }
            }
        }

        let visible_count = visible_verts.iter().filter(|&&v| v < 3).count();
        match visible_count {
            0 => None, // No vertices visible
            1 => {
                // If one vertex is visible, return the triangle with that vertex and two clipped edges
                let index_keep = visible_verts[0];
                let vert_in = verts[tri[index_keep]];
                let vert_out_a = verts[tri[(index_keep + 1) % 3]];
                let vert_out_b = verts[tri[(index_keep + 2) % 3]];

                let mut clip_a = vert_in;
                let mut clip_b = vert_in;
                // Find the intersection points with the frustum planes
                for i in 0..2 {
                    let v_out = match i {
                        0 => vert_out_a,
                        _ => vert_out_b,
                    };
                    
                    // Populate clip_v with the intersections of the edge with each plane that v does not pass
                    let mut clip_v: Vec<Vec3> = Vec::new();
                    for &planeidx in planes_not_passed[i].iter() {
                        let intersection = self.frustum_planes[planeidx].intersect_line(vert_in, v_out);
                        clip_v.push(intersection);
                    }
                    // Set clip_a and clip_b to the nearest intersection points
                    match i {
                        0 => clip_a = clip_v.into_iter().min_by(|&a, &b|
                                                                         (a - self.transform.location).length().partial_cmp(&(b - self.transform.location).length()).unwrap())
                                                                        .unwrap(),
                        _ => clip_b = clip_v.into_iter().min_by(|&a, &b|
                                                                         (a - self.transform.location).length().partial_cmp(&(b - self.transform.location).length()).unwrap())
                                                                        .unwrap(),
                    }
                }

                Some(vec![[vert_in, clip_a, clip_b]])
            },
            2 => {
                // If two vertices are visible, return the two triangles that make up the rectangle that is visible
                let index_a = visible_verts[0];
                let index_b = visible_verts[1];
                let a = verts[tri[index_a]];
                let b = verts[tri[index_b]];
                let vert_out = verts[tri[(3 - index_a - index_b) % 3]]; // The vertex that is not visible

                let mut clip_a = a;
                let mut clip_b = b;
                // Find the intersection points with the frustum planes
                for i in 0..2 {
                    let v_in = match i {
                        0 => a,
                        _ => b,
                    };
                    
                    // Populate clip_v with the intersections of the edge with each plane that vert_out does not pass
                    let mut clip_v: Vec<Vec3> = Vec::new();
                    for &planeidx in planes_not_passed[0].iter() {
                        let intersection = self.frustum_planes[planeidx].intersect_line(v_in, vert_out);
                        clip_v.push(intersection);
                    }
                    // Set clip_a and clip_b to the nearest intersection points
                    match i {
                        0 => clip_a = clip_v.into_iter().min_by(|&x, &y|
                                                                         (x - self.transform.location).length().partial_cmp(&(y - self.transform.location).length()).unwrap())
                                                                        .unwrap(),
                        _ => clip_b = clip_v.into_iter().min_by(|&x, &y|
                                                                         (x - self.transform.location).length().partial_cmp(&(y - self.transform.location).length()).unwrap())
                                                                        .unwrap(),
                    }
                }

                Some(vec![[a, b, clip_a], [clip_a, b, clip_b]])
            },
            _ => {
                // Since visible_count only has two elements, this case shouldn't be possible
                panic!("Unexpected number of visible vertices: {} verts of triangle ({}, {}, {}) visible", visible_count, v1, v2, v3);
            }
        }
    }

    pub fn project_tri(&self, tri: [Vec3; 3]) -> ([FPoint; 3], f32) {
        // For each vertex, calculate the corresponding screen location
        let p1 = self.project_point(self.transform.invtransform.project_point3(tri[0]));
        let p2 = self.project_point(self.transform.invtransform.project_point3(tri[1]));
        let p3 = self.project_point(self.transform.invtransform.project_point3(tri[2]));

        let centroid = (tri[0] + tri[1] + tri[2]) / 3.;
        let depth = self.transform.location.distance(centroid);

        // Place the three screenspace points in an array and return them
        let frag = [p1, p2, p3];
        return (frag, depth);
    }
    
    fn project_point(&self, v: Vec3) -> FPoint {
        let projv = self.proj_mat.project_point3(v);
        let dx = self.aspect_ratio*self.pixelscale*projv.x/projv.z;
        let dy = self.pixelscale*projv.y/projv.z;
        return FPoint::new(self.target_width as f32/2. + dx, self.target_height as f32/2. - dy);
    }
}

impl TransformComponent {

    pub fn new(location: Vec3) -> Self {
        let mut tmat = Mat4::IDENTITY;
        tmat.w_axis = location.extend(1.);
        let _rotmat = Mat4::IDENTITY;
        Self {transform: tmat, invtransform: tmat.inverse(), location: location, rotation: Vec3::ZERO, scale: Vec3::new(1., 1., 1.), forward: Vec3::new(0.,0.,1.), right: Vec3::new(1.,0.,0.)}
    }

    pub fn offset(&mut self, delta: Vec3) {
        let mut dmat = Mat4::IDENTITY;
        dmat.w_axis = delta.extend(1.);
        //println!("{}", self.location);
        self.location = dmat.project_point3(self.location);
        self.update_transform();
        //println!("{}", self.location);
    }

    // Why the heck is the x-axis controlling horizontal and the y-axis controlling vertical?
    // Like, it's working somehow but wtf is going on
    pub fn rotate(&mut self, /* 0 is X, 1 is Y, 2 is Z */ local_axis: i8, delta_degs: f32) {
        match local_axis {
            0 => {
                self.rotation.x -= delta_degs;
            },
            1 => {
                self.rotation.y += delta_degs;
            },
            2 => {
                self.rotation.z += delta_degs;
            },
            _ => panic!("Invalid local axis: please use 0 for x, 1 for y, or 2 for z")
        }

        self.update_transform();
    }

    pub fn update_transform(&mut self) {
        // Build translation, rotation (per axis), and scale matrices
        let translation = Mat4::from_translation(Vec3::from(self.location));
        let rotation_x = Mat4::from_rotation_x(self.rotation.x.to_radians());
        let rotation_y = Mat4::from_rotation_y(self.rotation.y.to_radians());
        let rotation_z = Mat4::from_rotation_z(self.rotation.z.to_radians());
        let scaling = Mat4::from_scale(Vec3::from(self.scale));

        self.transform = translation * scaling * rotation_z * rotation_y * rotation_x;

        self.invtransform = self.transform.inverse();
        self.forward = (rotation_z * rotation_y * rotation_x).transform_vector3(Vec3::new(0., 0., 1.));
        self.right = (rotation_z * rotation_y * rotation_x).transform_vector3(Vec3::new(1., 0., 0.));

        /*self.forward = self.rotation.project_point3(Vec3 {x: 0., y: 0., z: 1.});
        self.right = self.rotation.project_point3(Vec3 {x: 1., y: 0., z: 0.});*/
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

/* Global Functions */

pub fn read_geometry(path_prefix: &str, p: &str, vert_target: &mut Vec<Vec3>, tri_target: &mut Vec<[usize; 3]>, matIdcs_target: &mut Vec<usize>, mat_target: &mut Vec<Material>) {
    println!("reading...");

    // Materials
    let r_file_mats: std::fs::File = std::fs::File::open(path_prefix.to_owned() + p + ".mtl").expect(("mtl file '".to_owned() + p + ".mtl' not found at location\n" + path_prefix + p + ".mtl").as_str());
    let f_mats = BufReader::new(r_file_mats);
    let mut materials = HashMap::new();
    let mut mat_name: String = String::new();
    let mut mat_col = Material::DEFAULT.color;
    for line in f_mats.lines().filter_map(|result| result.ok()) {
        if !line.is_empty() {
            let segs: Vec<&str> = line.split(' ').collect();
            if segs[0] == "newmtl" {
                mat_name = segs[1].to_string().clone();
            }
            else if segs[0] == "Kd" {
                mat_col = Color::RGB((segs[1].parse::<f32>().unwrap() * 255.) as u8, (segs[2].parse::<f32>().unwrap() * 255.) as u8, (segs[3].parse::<f32>().unwrap() * 255.) as u8)
            }

            materials.insert(mat_name.clone(), Material { color: mat_col });
        }
    }

    // Create a vector of all the materials in the order they were read, for indexing during rendering
    for (_, m) in materials.iter() {
        mat_target.push(*m);
    }

    // From now on matName is the name of the currently referenced material,
    // used to key to a certain material in the Map.

    // Geometry
    let r_file_geo = std::fs::File::open(path_prefix.to_owned() + p + ".obj").expect("obj file not found!");
    let f_geo = BufReader::new(r_file_geo);
    for line in f_geo.lines().filter_map(|result| result.ok()) {
        if !line.is_empty() {
            let chars: Vec<char> = line.chars().collect();
            let segs: Vec<&str> = line.split(' ').collect();
            // Verts
            if chars[0] == 'v' && chars[1] == ' ' {
                vert_target.push(Vec3 { x: segs[1].parse().unwrap(), y: segs[2].parse().unwrap(), z: segs[3].parse().unwrap() });
            }
            // Tris (and assign Mats)
            else if chars[0] == 'f' && chars[1] == ' ' {
                let is1: Vec<&str> = segs[1].split('/').collect();
                let is2: Vec<&str> = segs[2].split('/').collect();
                let is3: Vec<&str> = segs[3].split('/').collect();
                tri_target.push([is1[0].parse::<usize>().unwrap() - 1,
                                 is2[0].parse::<usize>().unwrap() - 1,
                                 is3[0].parse::<usize>().unwrap() - 1]);

                // Find the index of the current material in the master map, and index
                // that position in the material indices vector.
                let mut current_idx: usize = 0;
                for (n, _) in materials.iter() {
                    if n == &mat_name {
                        matIdcs_target.push(current_idx);
                        break;
                    }
                    current_idx+=1;
                }
            }
            // Loading Mats
            else {
                if segs[0] == "usemtl" {
                    mat_name = segs[1].to_string();
                }
            }
        }
    }
    println!("done!");
}