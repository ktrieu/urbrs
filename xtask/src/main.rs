use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}

fn run() -> Result<(), String> {
    println!("ooga booga");

    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = Command::new(cargo)
        .current_dir(project_root())
        .args(&["run", "-p", "urbrs"])
        .status()
        .map_err(|e| e.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err("cargo run failed".to_string())
    }
}

fn try_main() -> Result<(), String> {
    let task = env::args()
        .nth(1)
        .ok_or("task argument is required".to_string())?;
    match task.as_str() {
        "run" => run(),
        _ => Err(format!("invalid task {}", task)),
    }
}

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
