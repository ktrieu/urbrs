use std::path::Path;

use gltf::{Gltf, Primitive, Semantic};

pub struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
}

pub struct Model {
    name: String,
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

pub enum ModelError {
    GltfError(gltf::Error),
    FormatError(&'static str),
}

impl From<gltf::Error> for ModelError {
    fn from(value: gltf::Error) -> Self {
        ModelError::GltfError(value)
    }
}

impl Model {
    pub fn new_from_gltf_file(path: &str) -> Result<Self, ModelError> {
        let (file, buffers, _) = gltf::import(path)?;

        let scene = file
            .default_scene()
            .ok_or(ModelError::FormatError("file had no default scene"))?;

        // Our scenes are very simple right now. Just grab the first mesh we find.
        let mut meshes = scene.nodes().filter_map(|n| n.mesh());

        let mesh = meshes
            .next()
            .ok_or(ModelError::FormatError("file had no meshes"))?;
        if meshes.next().is_some() {
            return Err(ModelError::FormatError("file had more than one mesh"));
        }

        // And grab the first primitive. Maybe later we can handle two of these?
        let mut primitives = mesh.primitives();
        let primitive = primitives
            .next()
            .ok_or(ModelError::FormatError("mesh had no primitives"))?;
        if primitives.next().is_some() {
            return Err(ModelError::FormatError("file had more than one primitive"))?;
        }

        let reader = primitive.reader(|prim_buffer| Some(&buffers[prim_buffer.index()]));

        let pos_iter = reader
            .read_positions()
            .ok_or(ModelError::FormatError("mesh had no positions"))?;
        let normal_iter = reader
            .read_positions()
            .ok_or(ModelError::FormatError("mesh had no normals"))?;

        let num_vertices = primitive
            .get(&Semantic::Positions)
            .ok_or(ModelError::FormatError("mesh had no positions"))?
            .count();

        let mut vertices: Vec<Vertex> = Vec::with_capacity(num_vertices);
        vertices.extend(
            pos_iter
                .zip(normal_iter)
                .map(|(position, normal)| Vertex { position, normal }),
        );

        let num_indices = primitive
            .indices()
            .ok_or(ModelError::FormatError("mesh had no indicies"))?
            .count();

        let mut indices: Vec<u32> = Vec::with_capacity(num_indices);

        indices.extend(
            reader
                .read_indices()
                .ok_or(ModelError::FormatError("mesh had no indices"))?
                .into_u32(),
        );

        let name: String = Path::new(path)
            .file_stem()
            .expect("glTF should be loaded from a path with filename")
            .to_string_lossy()
            .to_string();

        Ok(Self {
            name,
            vertices,
            indices,
        })
    }
}
