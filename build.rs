use std::error::Error;
use std::io::ErrorKind;
use std::process::Command;

#[cfg(target_os = "windows")]
const NPM_CMD: &str = "npm.cmd";

#[cfg(not(target_os = "windows"))]
const NPM_CMD: &str = "npm";

fn main() -> Result<(), Box<dyn Error>> {
    // Trigger recompilation when a new migration is added.
    println!("cargo:rerun-if-changed=migrations");

    // Trigger recompilation if frontend source is changed.
    println!("cargo:rerun-if-changed=frontend");

    // Run npm build.
    run_npm_command(&["install", "--include=\"dev\""])?;
    run_npm_command(&["run", "check"])?;
    run_npm_command(&["run", "build"])?;

    Ok(())
}

fn run_npm_command(args: &[&str]) -> Result<(), Box<dyn Error>> {
    let mut child = Command::new(NPM_CMD)
        .current_dir("frontend")
        .args(args)
        .spawn()
        .map_err(|e| match e.kind() {
            ErrorKind::NotFound => format!("npm is required to build the frontend but was not found"),
            _ => format!("Error when running building frontend ({NPM_CMD} {}): {e}", args.join(" ")),
        })?;

    if !child.wait()?.success() {
        Err(format!("Error when running {NPM_CMD} {}", args.join(" ")))?;
    }
    Ok(())
}
