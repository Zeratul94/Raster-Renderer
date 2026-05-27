#![allow(unused_must_use)]
#![allow(dead_code)]

extern crate sdl3;
extern crate glam;
extern crate rand;

use crate::gfx_engine;

use gfx_engine::ScreenTri;

use std::collections::HashMap;
use std::io::BufRead;
use std::io::BufReader;

use sdl3::render::FPoint;

use glam::Mat4;
use glam::Vec3;

use gfx_engine::Material;


/* Structs */

pub struct MaterialLibrary {
    pub materials: Vec<Material>,
    pub name_to_idx: HashMap<String, usize>,
}

#[derive(Clone, Copy)]
pub struct Plane {
    pub normal: Vec3,
    pub samplepoint: Vec3
}

#[derive(Clone, Copy)]
pub struct VertexData {
    pub normal: Vec3,
    pub position: Vec3,
    pub depth: f32
}

pub struct Mesh {
    pub transform: TransformComponent,
    verts: Vec<VertexData>,
    tris: Vec<[usize; 3]>,
    matIdcs: Vec<usize>,
}

pub struct Camera {
    pub transform: TransformComponent,
    pub focal_length: f32,
    proj_mat: Mat4,
    pub target_width: u32,
    pub target_height: u32,
    pub aspect_ratio: f32,
    pixelscale: f32,

    local_frustum_planes: [Plane; 6],
    pub world_frustum_planes: [Plane; 6]
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

impl MaterialLibrary {
    pub fn new() -> Self {
        Self {
            materials: Vec::new(),
            name_to_idx: HashMap::new(),
        }
    }

    pub fn get_or_add(&mut self, name: &str, material: Material) -> usize {
        if let Some(&idx) = self.name_to_idx.get(name) {
            idx
        } else {
            let idx = self.materials.len();
            self.materials.push(material);
            self.name_to_idx.insert(name.to_string(), idx);
            idx
        }
    }
}

impl Plane {
    // Intersects a line segment with the plane, returning the intersection point if there is one
    pub fn intersect_line(&self, line_start: Vec3, line_end: Vec3) -> Option<Vec3> {
        let line_slope = line_end - line_start;
        
        let t = -self.normal.dot(line_start - self.samplepoint) / self.normal.dot(line_slope);
        if t >= 0. && t <= 1. {Some(line_slope*t + line_start)} else {None}
    }
    // Intersects a line segment with the plane, returning the interpolation factor t at which the intersection occurs
    pub fn intersect_line_factor(&self, line_start: Vec3, line_end: Vec3) -> f32 {
        let line_slope = line_end - line_start;
        
        let t = -self.normal.dot(line_start - self.samplepoint) / self.normal.dot(line_slope);
        t
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

impl VertexData {
    fn lerp(self, rhs: VertexData, t: f32) -> Self {
        Self {
            normal: self.normal + (rhs.normal - self.normal) * t,
            position: self.position + (rhs.position - self.position) * t,
            depth: self.depth + (rhs.depth - self.depth) * t,
        }
    }
}

impl Mesh {
    pub fn from_obj(path_prefix: &str, p: &str, location: Vec3, mat_lib: &mut MaterialLibrary) -> Self {
        println!("reading {}...", p);

        let mut vert_target = Vec::new();
        let mut tri_target = Vec::new();
        let mut matIdcs_target = Vec::new();

        // Materials
        let mtl_path = format!("{}{}.mtl", path_prefix, p);
        let r_file_mats = std::fs::File::open(&mtl_path)
            .expect(&format!("mtl file '{}' not found at location {}", p, mtl_path));
        let f_mats = BufReader::new(r_file_mats);
        
        let mut current_mat_name: String = String::new();
        let mut temp_materials = HashMap::new();
        let mut mat_col = Material::DEFAULT.base_color;

        for line in f_mats.lines().filter_map(|result| result.ok()) {
            if !line.is_empty() {
                let segs: Vec<&str> = line.split_whitespace().collect();
                if segs.is_empty() { continue; }
                if segs[0] == "newmtl" {
                    current_mat_name = segs[1].to_string();
                } else if segs[0] == "Kd" {
                    mat_col = glam::Vec4::new(
                        segs[1].parse::<f32>().unwrap(),
                        segs[2].parse::<f32>().unwrap(),
                        segs[3].parse::<f32>().unwrap(),
                        1.0,
                    );
                }
                // Store the material under the current name in a temporary map for this OBJ
                temp_materials.insert(current_mat_name.clone(), Material { base_color: mat_col });
            }
        }

        // Geometry
        let obj_path = format!("{}{}.obj", path_prefix, p);
        let r_file_geo = std::fs::File::open(&obj_path).expect("obj file not found!");
        let f_geo = BufReader::new(r_file_geo);
        let mut vert_pos = Vec::new();
        let mut vert_norm = Vec::new();
        let mut active_mat_name = String::new();

        for line in f_geo.lines().filter_map(|result| result.ok()) {
            if line.is_empty() { continue; }
            let chars: Vec<char> = line.chars().collect();
            let segs: Vec<&str> = line.split_whitespace().collect();
            if segs.is_empty() { continue; }

            // Verts and Faces (usually starting with 'v ' or 'f ')
            if chars.len() > 1 && chars[1] == ' ' {
                match chars[0] {
                    'v' => {
                        vert_pos.push(Vec3 {
                            x: segs[1].parse().unwrap(),
                            y: segs[2].parse().unwrap(),
                            z: segs[3].parse().unwrap(),
                        });
                    }
                    'f' => {
                        let corners = &segs[1..4];
                        let start_idx = vert_target.len();

                        for corner in corners {
                            let parts: Vec<&str> = corner.split('/').collect();
                            let v_idx = parts[0].parse::<usize>().unwrap() - 1;
                            let vn_idx = if parts.len() >= 3 && !parts[2].is_empty() {
                                parts[2].parse::<usize>().unwrap() - 1
                            } else {
                                0
                            };

                            vert_target.push(VertexData {
                                position: vert_pos[v_idx],
                                normal: if vert_norm.is_empty() { Vec3::ZERO } else { vert_norm[vn_idx] },
                                depth: 0.0,
                            });
                        }

                        tri_target.push([start_idx, start_idx + 1, start_idx + 2]);

                        // Add material to global library if not already there, and store index
                        if let Some(mat) = temp_materials.get(&active_mat_name) {
                            matIdcs_target.push(mat_lib.get_or_add(&active_mat_name, *mat));
                        } else {
                            matIdcs_target.push(0); // Fallback
                        }
                    }
                    _ => {}
                }
            } else {
                // Normals and Material switches
                if segs[0] == "vn" {
                    vert_norm.push(Vec3 {
                        x: segs[1].parse().unwrap(),
                        y: segs[2].parse().unwrap(),
                        z: segs[3].parse().unwrap(),
                    });
                } else if segs[0] == "usemtl" {
                    active_mat_name = segs[1].to_string();
                }
            }
        }
        println!("done!");

        Self {
            transform: TransformComponent::new(location),
            verts: vert_target,
            tris: tri_target,
            matIdcs: matIdcs_target
        }
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

        let local_frustum_planes = [Plane { normal: (nw_dir.cross(ne_dir)).normalize(), samplepoint: nw },
                                         Plane { normal: (ne_dir.cross(se_dir)).normalize(), samplepoint: ne },
                                         Plane { normal: (se_dir.cross(sw_dir)).normalize(), samplepoint: se },
                                         Plane { normal: (sw_dir.cross(nw_dir)).normalize(), samplepoint: sw },
                                         Plane { normal: Vec3::new(0., 0., 1.), samplepoint: Vec3::new(0., 0., clip_dist) },
                                         Plane { normal: Vec3::new(0., 0., -1.), samplepoint: Vec3::new(0., 0., draw_dist) }];
        
        let mut cam = Self {transform: TransformComponent::new(location), focal_length: focal_length, proj_mat: pmat,
                                    target_width: target_width, target_height: target_height, aspect_ratio: aspect, pixelscale: 75., local_frustum_planes, world_frustum_planes: local_frustum_planes};
        cam.update_frustum_planes();
        cam
    }

    pub fn update_frustum_planes(&mut self) {
        for i in 0..6 {
            let local_plane = &self.local_frustum_planes[i];
            // Transform samplepoint to world space
            let world_samplepoint = self.transform.transform.project_point3(local_plane.samplepoint);
            // Transform normal to world space (only rotation matters for normals)
            let world_normal = (self.transform.transform.transform_vector3(local_plane.normal)).normalize();
            
            self.world_frustum_planes[i] = Plane { normal: world_normal, samplepoint: world_samplepoint };
        }
    }


    /* Gemini wrote this function */
    pub fn clip_tri_near_plane(&self, raw_tri: [usize; 3], verts: &Vec<VertexData>)  -> Vec<[VertexData; 3]> {
        let near_plane = &self.world_frustum_planes[4]; // Near Plane is at index 4
        
        // Check which vertices are "in" (dot product >= 0)
        let mut inside = Vec::new();
        let mut outside = Vec::new();
        let tri = [verts[raw_tri[0]], verts[raw_tri[1]], verts[raw_tri[2]]];

        for v in tri {
            if near_plane.normal.dot(v.position - near_plane.samplepoint) >= 0.0 {
                inside.push(v);
            } else {
                outside.push(v);
            }
        }

        match inside.len() {
            0 => vec![], // All outside
            3 => vec![tri], // All inside
            1 => {
                // One inside: Triangle becomes a smaller triangle
                let v_in = inside[0];
                let t1 = near_plane.intersect_line_factor(v_in.position, outside[0].position);
                let t2 = near_plane.intersect_line_factor(v_in.position, outside[1].position);
                vec![[v_in, v_in.lerp(outside[0], t1), v_in.lerp(outside[1], t2)]]
            }
            2 => {
                // Two inside: Triangle becomes a Quad (two triangles)
                let v_in1 = inside[0];
                let v_in2 = inside[1];
                let v_out = outside[0];
                let t1 = near_plane.intersect_line_factor(v_in1.position, v_out.position);
                let t2 = near_plane.intersect_line_factor(v_in2.position, v_out.position);
                
                let clip1 = v_in1.lerp(v_out, t1);
                let clip2 = v_in2.lerp(v_out, t2);
                
                vec![[v_in1, v_in2, clip1], [clip1, v_in2, clip2]]
            }
            _ => vec![],
        }
    }

    /* DOESN'T REALLY WORK */
    // Check if a triangle is wholly or partially in the view frustum, and return one of three options:
    // 1. If 0 vertices are visible, return None.
    // 2. If 1 or 3 vertices are contained, return Some(triangle) with the triangle or the version of it
    // clipped to the frustum.
    // 3. If 2 vertices are contained, return the two triangles that make up the rect that is visible.
    pub fn clip_tri_to_frustum(&self, tri: [usize; 3], verts: &Vec<VertexData>) -> Option<Vec<[VertexData; 3]>> {
        // Get the vertices of the triangle
        let v1 = verts[tri[0]].position;
        let v2 = verts[tri[1]].position;
        let v3 = verts[tri[2]].position;

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
            for plane in self.world_frustum_planes.iter() {
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
                    return Some(vec![[verts[tri[0]], verts[tri[1]], verts[tri[2]]]]); // If both slots are filled and the third vertex is also visible, the triangle is fully visible
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
                    let mut clip_v: Vec<VertexData> = Vec::new();
                    for &planeidx in planes_not_passed[i].iter() {
                        let t = self.world_frustum_planes[planeidx].intersect_line_factor(vert_in.position, v_out.position);
                        clip_v.push(vert_in.lerp(v_out, t));
                    }
                    // Set clip_a and clip_b to the nearest intersection points
                    match i {
                        0 => clip_a = clip_v.into_iter().min_by(|&a, &b|
                                                                         (a.position - self.transform.location).length().partial_cmp(&(b.position - self.transform.location).length()).unwrap())
                                                                        .unwrap(),
                        _ => clip_b = clip_v.into_iter().min_by(|&a, &b|
                                                                         (a.position - self.transform.location).length().partial_cmp(&(b.position - self.transform.location).length()).unwrap())
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
                    let mut clip_v: Vec<VertexData> = Vec::new();
                    for &planeidx in planes_not_passed[0].iter() {
                        let t = self.world_frustum_planes[planeidx].intersect_line_factor(v_in.position, vert_out.position);
                        clip_v.push(v_in.lerp(vert_out, t));
                    }
                    // Set clip_a and clip_b to the nearest intersection points
                    match i {
                        0 => clip_a = clip_v.into_iter().min_by(|&x, &y|
                                                                         (x.position - self.transform.location).length().partial_cmp(&(y.position - self.transform.location).length()).unwrap())
                                                                        .unwrap(),
                        _ => clip_b = clip_v.into_iter().min_by(|&x, &y|
                                                                         (x.position - self.transform.location).length().partial_cmp(&(y.position - self.transform.location).length()).unwrap())
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

    pub fn project_mesh(&self, mesh: &Mesh) -> Vec<ScreenTri> {
        // Project geometry to the screen
        // Each screen triangle is a tuple of: screenspace vertices, centroid depth, material index, 3D vertex data])
        let mut screen_tris = Vec::new();
        let verts= mesh.verts.iter()
                                                .map(|v| VertexData {normal: mesh.transform.transform.transform_vector3(v.normal), depth: 0., position: mesh.transform.transform.project_point3(v.position)})
                                                .collect::<Vec<VertexData>>();
        for i in 0..mesh.tris.len() {
            let tri_verts = [verts[mesh.tris[i][0]], verts[mesh.tris[i][1]], verts[mesh.tris[i][2]]];
            
            // Backface Culling: Calculate normal and check if it faces the camera
            let edge1 = tri_verts[1].position - tri_verts[0].position;
            let edge2 = tri_verts[2].position - tri_verts[0].position;
            let tri_normal = edge1.cross(edge2);
            let view_dir = tri_verts[0].position - self.transform.location;

            if tri_normal.dot(view_dir) >= 0. { continue; }

            let clipped_tri = self.clip_tri_near_plane(mesh.tris[i], &verts);
            for tri in clipped_tri.iter() {
                let (screen_tri, cdepth, vdepths) = self.project_tri(tri.map(|v| v.position));

                screen_tris.push((screen_tri, cdepth, mesh.matIdcs[i], [VertexData { normal: tri[0].normal, position: tri[0].position, depth: vdepths[0] },
                                                                    VertexData { normal: tri[1].normal, position: tri[1].position, depth: vdepths[1] },
                                                                    VertexData { normal: tri[2].normal, position: tri[2].position, depth: vdepths[2] }]));
            }
        }

        screen_tris
    }

    pub fn project_tri(&self, tri: [Vec3; 3]) -> ([FPoint; 3], f32, [f32; 3]) {
        // For each vertex, calculate the corresponding screen location
        let (p1, depth1) = self.project_point(self.transform.invtransform.project_point3(tri[0]));
        let (p2, depth2) = self.project_point(self.transform.invtransform.project_point3(tri[1]));
        let (p3, depth3) = self.project_point(self.transform.invtransform.project_point3(tri[2]));

        let vdepths = [depth1, depth2, depth3];
        let cdepth = (depth1 + depth2 + depth3) / 3.0;

        // Place the three screenspace points in an array and return them
        let frag = [p1, p2, p3];
        return (frag, cdepth, vdepths);
    }
    
    fn project_point(&self, v: Vec3) -> (FPoint, f32) {
        // Direct perspective projection instead of using broken proj_mat
        let dx = self.aspect_ratio * self.pixelscale * self.focal_length * v.x / v.z;
        let dy = self.pixelscale * self.focal_length * v.y / v.z;
        return (FPoint::new(self.target_width as f32/2. + dx, self.target_height as f32/2. - dy), v.z);
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
        self.location = dmat.project_point3(self.location);
        self.update_transform();
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

pub fn read_geometry(path_prefix: &str, p: &str, vert_target: &mut Vec<VertexData>, tri_target: &mut Vec<[usize; 3]>, matIdcs_target: &mut Vec<usize>, mat_target: &mut Vec<Material>) {
    println!("reading...");

    // Materials
    let r_file_mats: std::fs::File = std::fs::File::open(path_prefix.to_owned() + p + ".mtl").expect(("mtl file '".to_owned() + p + ".mtl' not found at location\n" + path_prefix + p + ".mtl").as_str());
    let f_mats = BufReader::new(r_file_mats);
    let mut materials = HashMap::new();
    let mut vert_pos = Vec::new();
    let mut vert_norm = Vec::new();
    let mut mat_name: String = String::new();
    let mut mat_col = Material::DEFAULT.base_color;
    for line in f_mats.lines().filter_map(|result| result.ok()) {
        if !line.is_empty() {
            let segs: Vec<&str> = line.split(' ').collect();
            if segs[0] == "newmtl" {
                mat_name = segs[1].to_string().clone();
            }
            else if segs[0] == "Kd" {
                mat_col = glam::Vec4::new(segs[1].parse::<f32>().unwrap(), segs[2].parse::<f32>().unwrap(), segs[3].parse::<f32>().unwrap(), 1.)
            }

            materials.insert(mat_name.clone(), Material { base_color: mat_col });
        }
    }

    // Create a vector of all the materials and a map for their indices
    let mut mat_name_to_idx = HashMap::new();
    for (name, m) in materials.iter() {
        mat_name_to_idx.insert(name.clone(), mat_target.len());
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
            if chars[1] == ' ' {
                match chars[0] {
                    'v' => {
                        vert_pos.push(Vec3 { x: segs[1].parse().unwrap(), y: segs[2].parse().unwrap(), z: segs[3].parse().unwrap() });
                    },
                    'f' => {
                        /* Gemini block */
                        let corners = [segs[1], segs[2], segs[3]];
                        let start_idx = vert_target.len();

                        for corner in corners {
                            let parts: Vec<&str> = corner.split('/').collect();
                            
                            // 1. Get the position index (first part)
                            let v_idx = parts[0].parse::<usize>().unwrap() - 1;
                            
                            // 2. Get the normal index (third part, if it exists)
                            let vn_idx = if parts.len() >= 3 && !parts[2].is_empty() {
                                parts[2].parse::<usize>().unwrap() - 1
                            } else {
                                0 // Fallback if the file is missing normals
                            };

                            // 3. Weld them into a single unique VertexData
                            vert_target.push(VertexData {
                                position: vert_pos[v_idx],
                                normal: if vert_norm.is_empty() { Vec3::ZERO } else { vert_norm[vn_idx] },
                                depth: 0.0,
                            });
                        }

                        // 4. Triangle indices always point to the 3 most recently added vertices
                        tri_target.push([start_idx, start_idx + 1, start_idx + 2]);
                        /* end of Gemini block */

                        // Find the index of the current material in the master map, and index
                        // that position in the material indices vector.
                        if let Some(&idx) = mat_name_to_idx.get(&mat_name) {
                            matIdcs_target.push(idx);
                        } else {
                            matIdcs_target.push(0); // Fallback
                        }
                    },
                    _ => {}
                }
            // Loading Mats
            }
            else {
                if segs[0] == "vn" {
                    vert_norm.push(Vec3 { x: segs[1].parse().unwrap(), y: segs[2].parse().unwrap(), z: segs[3].parse().unwrap() });
                }
                if segs[0] == "usemtl" {
                    mat_name = segs[1].to_string();
                }
            }
        }
    }
    println!("done!");
}