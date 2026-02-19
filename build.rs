use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=static/css/input.css");
    println!("cargo:rerun-if-changed=templates/");

    let status = Command::new("npx")
        .args([
            "@tailwindcss/cli",
            "-i",
            "static/css/input.css",
            "-o",
            "static/css/dist/output.css",
            "--minify",
        ])
        .status();

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            eprintln!("cargo:warning=Tailwind CSS build exited with: {s}");
        }
        Err(e) => {
            eprintln!("cargo:warning=Failed to run Tailwind CSS: {e}");
        }
    }
}
