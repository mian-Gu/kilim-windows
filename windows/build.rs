fn main() {
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        let root = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
        let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        let version = std::env::var("CARGO_PKG_VERSION").unwrap();
        let numeric = version.split('.').take(3).collect::<Vec<_>>().join(",") + ",0";
        std::fs::write(
            out.join("version.h"),
            format!(
                "#define KILIM_FILEVERSION {numeric}\n#define KILIM_VERSIONTEXT \"{version}\"\n"
            ),
        )
        .unwrap();
        let compiler = resource_compiler()
            .expect("Windows SDK rc.exe is required to build the application icon resources");
        let resource = out.join("kilim.res");
        let status = std::process::Command::new(compiler)
            .arg("/nologo")
            .arg("/fo")
            .arg(&resource)
            .arg("/i")
            .arg(&out)
            .arg(root.join("app.rc"))
            .current_dir(&root)
            .status()
            .expect("Could not run Windows resource compiler");
        assert!(status.success(), "Windows resource compilation failed");
        println!("cargo:rustc-link-arg={}", resource.display());
        let manifest = std::path::Path::new("app.manifest").canonicalize().unwrap();
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
        println!("cargo:rerun-if-changed=app.manifest");
        for file in ["app.rc", "assets/app.ico", "assets/grab.cur"] {
            println!("cargo:rerun-if-changed={file}");
        }
    }
}
fn resource_compiler() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    let mut roots = Vec::new();
    if let Some(root) = std::env::var_os("WindowsSdkDir") {
        roots.push(PathBuf::from(root).join("bin"));
    }
    if let Some(root) = std::env::var_os("ProgramFiles(x86)") {
        roots.push(PathBuf::from(root).join("Windows Kits/10/bin"));
    }
    let mut candidates = Vec::new();
    for root in roots {
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                let path = entry.path().join("x64/rc.exe");
                if path.is_file() {
                    let version = entry
                        .file_name()
                        .to_string_lossy()
                        .split('.')
                        .filter_map(|part| part.parse::<u32>().ok())
                        .collect::<Vec<_>>();
                    candidates.push((version, path));
                }
            }
        }
    }
    candidates
        .into_iter()
        .max_by_key(|(version, _)| version.clone())
        .map(|(_, path)| path)
}
