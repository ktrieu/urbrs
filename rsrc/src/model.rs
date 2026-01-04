use std::{fmt::Display, path::Path};

use common::{Model, Vertex};
use gltf::{mesh::Mode, Semantic};

pub enum ModelError {
    GltfError(gltf::Error),
    FormatError(&'static str),
}

impl From<gltf::Error> for ModelError {
    fn from(value: gltf::Error) -> Self {
        ModelError::GltfError(value)
    }
}

impl Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelError::GltfError(error) => write!(f, "glTF load error: {error}"),
            ModelError::FormatError(s) => write!(f, "model file format error: {s}"),
        }
    }
}

pub fn new_model_from_gltf_file(path: &Path) -> Result<Model, ModelError> {
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

    if primitive.mode() != Mode::Triangles {
        return Err(ModelError::FormatError(
            "primitive was not in triangle format",
        ));
    }

    let reader = primitive.reader(|prim_buffer| Some(&buffers[prim_buffer.index()]));

    let pos_iter = reader
        .read_positions()
        .ok_or(ModelError::FormatError("mesh had no positions"))?;
    let normal_iter = reader
        .read_normals()
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

    let name: String = path
        .file_stem()
        .ok_or(ModelError::FormatError(
            "glTF should be loaded from a path with filename",
        ))?
        .to_string_lossy()
        .to_string();

    Ok(Model {
        name,
        vertices,
        indices,
    })
}
