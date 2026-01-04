use std::{
    fmt::Display,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    process::exit,
};

use rkyv::rancor;
use walkdir::WalkDir;

use crate::model::{new_model_from_gltf_file, ModelError};

mod model;

enum RsrcError {
    IoError(io::Error),
    ModelError(model::ModelError),
    RkyvError(rancor::Error),
    Other(String),
}

impl From<io::Error> for RsrcError {
    fn from(value: io::Error) -> Self {
        RsrcError::IoError(value)
    }
}

impl From<walkdir::Error> for RsrcError {
    fn from(value: walkdir::Error) -> Self {
        RsrcError::IoError(value.into())
    }
}

impl From<String> for RsrcError {
    fn from(value: String) -> Self {
        RsrcError::Other(value)
    }
}

impl From<ModelError> for RsrcError {
    fn from(value: ModelError) -> Self {
        RsrcError::ModelError(value)
    }
}

impl Display for RsrcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RsrcError::IoError(error) => write!(f, "io error: {error}"),
            RsrcError::ModelError(error) => write!(f, "model load error: {error}"),
            RsrcError::RkyvError(error) => write!(f, "rkyv error: {error}"),
            RsrcError::Other(s) => write!(f, "{s}"),
        }
    }
}

type RsrcResult<T> = Result<T, RsrcError>;

fn get_shader_output_path(path: &Path, ext: &str) -> PathBuf {
    let ext = "spv.".to_string() + ext;

    path.with_extension(ext)
}

fn get_output_rel_path(path: &Path) -> RsrcResult<PathBuf> {
    let ext = path.extension().map(|os_str| os_str.to_str()).flatten();

    Ok(match ext {
        Some("vert") => get_shader_output_path(path, "vert"),
        Some("frag") => get_shader_output_path(path, "frag"),
        Some("glb") => path.with_extension("mdl"),
        _ => path.to_path_buf(),
    })
}

fn glslc_compile(source: &Path, dest: &Path) -> RsrcResult<()> {
    let output = std::process::Command::new("glslc")
        .arg("-o")
        .arg(dest.as_os_str())
        .arg(source.as_os_str())
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(stderr.into())
    } else {
        Ok(())
    }
}

fn gltf_process(source: &Path, dest: &Path) -> RsrcResult<()> {
    let model = new_model_from_gltf_file(source)?;

    let bytes = rkyv::to_bytes::<rancor::Error>(&model).map_err(|e| RsrcError::RkyvError(e))?;

    File::create(dest)?.write_all(&bytes)?;

    Ok(())
}

fn basic_copy(source: &Path, dest: &Path) -> RsrcResult<()> {
    fs::copy(source, dest)?;

    Ok(())
}

fn should_skip_process(source: &Path, dest: &Path) -> RsrcResult<bool> {
    if !fs::exists(dest)? {
        return Ok(false);
    }

    let source_mtime = fs::metadata(source)?.modified()?;
    let dest_mtime = fs::metadata(dest)?.modified()?;

    Ok(source_mtime <= dest_mtime)
}

fn process(source: &Path, dest: &Path) -> RsrcResult<()> {
    if should_skip_process(source, dest)? {
        return Ok(());
    }

    let ext = source.extension().map(|os_str| os_str.to_str()).flatten();

    match ext {
        Some("vert") => glslc_compile(source, dest),
        Some("frag") => glslc_compile(source, dest),
        Some("glb") => gltf_process(source, dest),
        // No-op, we don't want to process these.
        Some("blend") | Some("blend1") => Ok(()),
        _ => basic_copy(source, dest),
    }
}

fn rsrc_main() -> RsrcResult<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        return Err(RsrcError::Other("invalid number of arguments".to_string()));
    }

    let source_dir = Path::new(&args[1]);
    let out_dir = Path::new(&args[2]);

    let walk = WalkDir::new(source_dir);
    for entry in walk {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let source = entry.path();

        let rel = entry.path().strip_prefix(source_dir).map_err(|_e| {
            format!(
                "could not calculate relative path for {}",
                entry.path().display()
            )
            .to_string()
        })?;

        let output_rel_path = get_output_rel_path(rel)?;

        let dest = out_dir.join(output_rel_path);
        println!("{} -> {}", entry.path().display(), dest.display());

        // Make sure our output dir exists before processing.
        if let Some(dir) = dest.parent() {
            fs::create_dir_all(dir)?;
        }
        process(source, &dest)?;
    }

    Ok(())
}

fn main() {
    let result = rsrc_main();

    if let Err(e) = result {
        eprintln!("error: {e}");
        exit(1);
    }
}
