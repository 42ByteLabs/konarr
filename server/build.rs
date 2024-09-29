use std::process::Command;

fn main() {
    // Git commit hash
    let git_output = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .expect("Failed to execute git command");
    let commit = String::from_utf8(git_output.stdout).unwrap();

    println!("cargo:rustc-env=KONARR_GIT_COMMIT={}", commit);
}
