//@ needs-llvm-components: riscv

#![deny(warnings)]

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use run_make_support::{cargo, cmd, llvm_filecheck, path, rfs};

const TARGET: &str = "riscv64a23-unknown-linux-gnu";
const DYNAMIC_LINKER: &str = "ld-linux-riscv64-lp64d.so.1";

fn main() {
    let target_dir = path("target");

    check_assembly(&target_dir);

    let tools = match RuntimeTools::discover() {
        Ok(Some(tools)) => tools,
        Ok(None) => {
            eprintln!(
                "skipping RV64 runtime checks: set RISCV64_QEMU and \
                 RISCV64_LINUX_GNU_GCC (and optionally RISCV64_SYSROOT), or put \
                 qemu-riscv64 and a RISC-V Linux GNU compiler in PATH"
            );
            return;
        }
        Err(error) => panic!("invalid RV64 runtime tool configuration: {error}"),
    };

    check_runtime(&target_dir, &tools);
}

fn check_assembly(target_dir: &Path) {
    // Rebuild core for an RVV baseline so that its target-feature cfg selects
    // the real SIMD implementation behind the public str::contains method.
    cargo()
        .args(&[
            "rustc",
            "--release",
            "--lib",
            "--manifest-path",
            "Cargo.toml",
            "-Zbuild-std=core",
            "--target",
            TARGET,
            "--",
            "--emit=asm",
            "-Ccodegen-units=1",
        ])
        .env("CARGO_TARGET_DIR", target_dir)
        .env("RUSTC_BOOTSTRAP", "1")
        .run();

    let out_dir = target_dir.join(TARGET).join("release").join("deps");
    let asm_file = rfs::read_dir(out_dir)
        .find_map(|entry| {
            let path = entry.unwrap().path();
            let file_name = path.file_name().unwrap().to_str().unwrap();
            if file_name.starts_with("str_contains_riscv-") && file_name.ends_with(".s") {
                Some(path)
            } else {
                None
            }
        })
        .unwrap();

    llvm_filecheck().patterns("lib.rs").input_file(asm_file).run();
}

fn check_runtime(target_dir: &Path, tools: &RuntimeTools) {
    cargo()
        .args(&[
            "rustc",
            "--release",
            "--bin",
            "str_contains_riscv_runtime",
            "--manifest-path",
            "Cargo.toml",
            "-Zbuild-std=std",
            "--target",
            TARGET,
        ])
        .env("CARGO_TARGET_DIR", target_dir)
        .env("CARGO_TARGET_RISCV64A23_UNKNOWN_LINUX_GNU_LINKER", &tools.gcc)
        .env("RUSTC_BOOTSTRAP", "1")
        .run();

    let executable = target_dir.join(TARGET).join("release").join("str_contains_riscv_runtime");
    for vlen in [128, 256, 512] {
        cmd(&tools.qemu)
            .arg("-cpu")
            .arg(format!("max,v=true,vlen={vlen}"))
            .arg("-L")
            .arg(&tools.sysroot)
            .arg(&executable)
            .run();
    }
}

struct RuntimeTools {
    qemu: PathBuf,
    gcc: PathBuf,
    sysroot: PathBuf,
}

impl RuntimeTools {
    fn discover() -> Result<Option<Self>, String> {
        let Some(qemu) = find_tool("RISCV64_QEMU", &["qemu-riscv64"])? else {
            return Ok(None);
        };
        let Some(gcc) = find_tool(
            "RISCV64_LINUX_GNU_GCC",
            &["riscv64-linux-gnu-gcc", "riscv64-unknown-linux-gnu-gcc"],
        )?
        else {
            return Ok(None);
        };
        validate_gcc_target(&gcc)?;

        let sysroot = match env::var_os("RISCV64_SYSROOT") {
            Some(path) => validate_sysroot(PathBuf::from(path), "RISCV64_SYSROOT")?,
            None => {
                let Some(sysroot) = compiler_sysroot(&gcc) else {
                    return Ok(None);
                };
                sysroot
            }
        };

        Ok(Some(Self { qemu, gcc, sysroot }))
    }
}

fn find_tool(env_name: &str, candidates: &[&str]) -> Result<Option<PathBuf>, String> {
    if let Some(tool) = env::var_os(env_name) {
        let tool = PathBuf::from(tool);
        if !executable_works(&tool) {
            return Err(format!("{env_name}={} is not an executable tool", tool.display()));
        }
        return Ok(Some(tool));
    }

    Ok(candidates.iter().find_map(|candidate| find_in_path(candidate)))
}

fn find_in_path(executable: &str) -> Option<PathBuf> {
    env::split_paths(&env::var_os("PATH")?).find_map(|dir| {
        let candidate = dir.join(executable);
        executable_works(&candidate).then_some(candidate)
    })
}

fn executable_works(path: &Path) -> bool {
    path.is_file()
        && Command::new(path).arg("--version").output().is_ok_and(|output| output.status.success())
}

fn compiler_sysroot(gcc: &Path) -> Option<PathBuf> {
    if let Some(reported) = compiler_print_path(gcc, "-print-sysroot") {
        if let Ok(reported) = validate_sysroot(reported, "compiler-reported sysroot") {
            return Some(reported);
        }
    }

    // Debian cross compilers commonly report `/` as their sysroot while keeping target files
    // below `/usr/riscv64-linux-gnu`. Ask GCC where the runtime loader actually lives so QEMU's
    // `-L` prefix still points at the target root in that configuration.
    let linker = compiler_print_path(gcc, &format!("-print-file-name={DYNAMIC_LINKER}"))?;
    let root = linker.parent()?.parent()?.to_path_buf();
    validate_sysroot(root, "compiler runtime root").ok()
}

fn validate_gcc_target(gcc: &Path) -> Result<(), String> {
    let output = Command::new(gcc)
        .arg("-dumpmachine")
        .output()
        .map_err(|error| format!("failed to query {}: {error}", gcc.display()))?;
    if !output.status.success() {
        return Err(format!("{} -dumpmachine failed", gcc.display()));
    }

    let machine = String::from_utf8(output.stdout)
        .map_err(|error| format!("{} returned a non-UTF-8 target: {error}", gcc.display()))?;
    if !machine.trim().starts_with("riscv64") {
        return Err(format!("{} targets {}, not riscv64", gcc.display(), machine.trim()));
    }
    Ok(())
}

fn compiler_print_path(gcc: &Path, argument: &str) -> Option<PathBuf> {
    let output = Command::new(gcc).arg(argument).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let path = String::from_utf8(output.stdout).ok()?;
    let path = PathBuf::from(path.trim());
    (!path.as_os_str().is_empty()).then_some(path)
}

fn validate_sysroot(path: PathBuf, source: &str) -> Result<PathBuf, String> {
    let path = path
        .canonicalize()
        .map_err(|error| format!("{source} {} is not accessible: {error}", path.display()))?;
    if !has_dynamic_linker(&path) {
        return Err(format!(
            "{source} {} does not contain lib/{DYNAMIC_LINKER} or lib64/{DYNAMIC_LINKER}",
            path.display()
        ));
    }
    Ok(path)
}

fn has_dynamic_linker(sysroot: &Path) -> bool {
    sysroot.join("lib").join(DYNAMIC_LINKER).is_file()
        || sysroot.join("lib64").join(DYNAMIC_LINKER).is_file()
}
