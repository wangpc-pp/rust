//! Exercise the RISC-V vector C ABI across real Rust and Clang objects. RV64 additionally links
//! with a cross GCC and runs under QEMU when those external tools are available; RV32 remains an
//! object/IR check because the commonly used Linux toolchain has no RV32 multilib. Set
//! `RVV_ABI_INTEROP_REQUIRE_RUNTIME=1` in a local run to make missing RV64 runtime tools fatal.

//@ needs-force-clang-based-tests
//@ needs-llvm-components: riscv

use std::env;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::Command as StdCommand;

use run_make_support::{clang, cmd, llvm_readobj, path, rfs, rustc, rustc_minicore};

const RUST_VARIANT_SYMBOLS: &[&str] =
    &["rust_vector_identity", "rust_tuple_identity", "rust_calls_c_vector", "rust_calls_c_tuple"];

const C_VARIANT_SYMBOLS: &[&str] = &["c_vector_add", "c_tuple_add"];

fn main() {
    for target in [
        Target {
            name: "rv32",
            rust: "riscv32gc-unknown-linux-gnu",
            clang: "riscv32-unknown-linux-gnu",
            march: "rv32gcv",
            mabi: "ilp32d",
            elf_class: "ELF32",
            xlen_type: "i32",
        },
        Target {
            name: "rv64",
            rust: "riscv64gc-unknown-linux-gnu",
            clang: "riscv64-unknown-linux-gnu",
            march: "rv64gcv",
            mabi: "lp64d",
            elf_class: "ELF64",
            xlen_type: "i64",
        },
    ] {
        build_and_check(target);
    }

    run_rv64_if_available();
}

#[derive(Clone, Copy)]
struct Target {
    name: &'static str,
    rust: &'static str,
    clang: &'static str,
    march: &'static str,
    mabi: &'static str,
    elf_class: &'static str,
    xlen_type: &'static str,
}

fn build_and_check(target: Target) {
    let minicore = format!("libminicore-{}.rlib", target.name);
    rustc_minicore().target(target.rust).panic("abort").output(&minicore).run();

    let rust_object = format!("rust-{}.o", target.name);
    let rust_ir = format!("rust-{}.ll", target.name);
    rustc()
        .input("rust.rs")
        .target(target.rust)
        .panic("abort")
        .opt_level("2")
        .arg("-Dimproper-ctypes")
        .arg("-Dimproper-ctypes-definitions")
        .arg("-Ctarget-feature=+v")
        .arg("-Zmerge-functions=disabled")
        .extern_("minicore", path(&minicore))
        .emit(format!("obj={rust_object},llvm-ir={rust_ir}"))
        .run();

    let c_object = format!("c-{}.o", target.name);
    clang_for(target)
        .arg("-O2")
        .arg("-ffreestanding")
        .arg("-c")
        .input("interop.c")
        .arg("-o")
        .arg(&c_object)
        .run();

    let c_ir = format!("c-{}.ll", target.name);
    clang_for(target)
        .arg("-O2")
        .arg("-ffreestanding")
        .arg("-S")
        .arg("-emit-llvm")
        .input("interop.c")
        .arg("-o")
        .arg(&c_ir)
        .run();

    check_elf(&rust_object, target.elf_class, RUST_VARIANT_SYMBOLS, &[]);
    check_elf(&c_object, target.elf_class, C_VARIANT_SYMBOLS, &["main"]);

    // Both frontends must describe the exact same public ABI types. This is especially useful on
    // RV32, where the test intentionally does not claim runtime coverage.
    let tuple_type = r#"target("riscv.vector.tuple", <vscale x 8 x i8>, 2)"#;
    let vector_type = "<vscale x 2 x i32>";
    for (ir, symbol, ty) in [
        (&rust_ir, "rust_vector_identity", vector_type),
        (&rust_ir, "c_vector_add", vector_type),
        (&rust_ir, "rust_tuple_identity", tuple_type),
        (&rust_ir, "c_tuple_add", tuple_type),
        (&c_ir, "rust_vector_identity", vector_type),
        (&c_ir, "c_vector_add", vector_type),
        (&c_ir, "rust_tuple_identity", tuple_type),
        (&c_ir, "c_tuple_add", tuple_type),
    ] {
        check_ir_arg_and_return(ir, symbol, ty);
    }
    for (ir, symbol) in [
        (&rust_ir, "c_vector_add"),
        (&rust_ir, "c_tuple_add"),
        (&c_ir, "c_vector_add"),
        (&c_ir, "c_tuple_add"),
    ] {
        check_last_ir_arg(ir, symbol, target.xlen_type);
    }
}

fn clang_for(target: Target) -> run_make_support::Clang {
    let mut command = clang();
    command.target(target.clang).arch(target.march).arg(format!("-mabi={}", target.mabi));
    command
}

fn check_elf(object: &str, elf_class: &str, variant: &[&str], ordinary: &[&str]) {
    let header = llvm_readobj().input(object).file_header().run().stdout_utf8();
    assert!(
        header.contains(elf_class) && header.contains("RISC-V"),
        "unexpected ELF header for {object}:\n{header}"
    );

    let symbols = llvm_readobj().input(object).symbols().run().stdout_utf8();
    for symbol in variant {
        assert_variant_cc(&symbols, symbol, true);
    }
    for symbol in ordinary {
        assert_variant_cc(&symbols, symbol, false);
    }
}

#[track_caller]
fn assert_variant_cc(symbols: &str, name: &str, expected: bool) {
    let line = symbols
        .lines()
        .find(|line| line.split_whitespace().last() == Some(name))
        .unwrap_or_else(|| panic!("missing symbol `{name}` in:\n{symbols}"));
    assert_eq!(
        line.contains("[VARIANT_CC]"),
        expected,
        "unexpected STO_RISCV_VARIANT_CC state for `{name}` in:\n{line}"
    );
}

#[track_caller]
fn ir_signature(ir_file: &str, symbol: &str) -> String {
    let marker = format!("@{symbol}(");
    rfs::read_to_string(ir_file)
        .lines()
        .find(|line| {
            (line.starts_with("define ") || line.starts_with("declare ")) && line.contains(&marker)
        })
        .unwrap_or_else(|| panic!("missing function `{symbol}` in {ir_file}"))
        .to_owned()
}

#[track_caller]
fn check_ir_arg_and_return(ir_file: &str, symbol: &str, expected_type: &str) {
    let signature = ir_signature(ir_file, symbol);
    assert_eq!(
        signature.matches(expected_type).count(),
        2,
        "wrong `{expected_type}` ABI for `{symbol}` in {ir_file}:\n{signature}",
    );
}

#[track_caller]
fn check_last_ir_arg(ir_file: &str, symbol: &str, expected_type: &str) {
    let signature = ir_signature(ir_file, symbol);
    let last_arg = signature.rsplit(',').next().unwrap();
    assert!(
        last_arg.contains(expected_type),
        "`{symbol}` must use `{expected_type}` for its final argument in {ir_file}:\n{signature}"
    );
}

fn run_rv64_if_available() {
    let required = env_flag("RVV_ABI_INTEROP_REQUIRE_RUNTIME");
    let Some(gcc) =
        find_tool("RISCV64_GCC", &["riscv64-unknown-linux-gnu-gcc", "riscv64-linux-gnu-gcc"])
    else {
        runtime_unavailable(required, "no RISC-V 64-bit Linux cross GCC found");
        return;
    };
    let Some(qemu) = find_tool("QEMU_RISCV64", &["qemu-riscv64"]) else {
        runtime_unavailable(required, "qemu-riscv64 was not found");
        return;
    };
    let Some(sysroot) = gcc_sysroot(&gcc) else {
        runtime_unavailable(required, "the RISC-V cross GCC has no usable sysroot");
        return;
    };

    cmd(&gcc)
        .arg("-march=rv64gcv")
        .arg("-mabi=lp64d")
        .arg("c-rv64.o")
        .arg("rust-rv64.o")
        .arg("-o")
        .arg("riscv-vector-abi-interop")
        .run();

    for vlen in [128, 256] {
        cmd(&qemu)
            .arg("-L")
            .arg(&sysroot)
            .arg("-cpu")
            .arg(format!("max,v=true,vlen={vlen}"))
            .arg(path("riscv-vector-abi-interop"))
            .run();
    }
}

fn env_flag(name: &str) -> bool {
    let Some(value) = env::var_os(name) else {
        return false;
    };
    match value.to_string_lossy().to_ascii_lowercase().as_str() {
        "1" | "yes" | "on" => true,
        "" | "0" | "no" | "off" => false,
        value => panic!("unrecognized boolean `{value}` in {name}"),
    }
}

fn find_tool(env_name: &str, fallbacks: &[&str]) -> Option<OsString> {
    if let Some(tool) = env::var_os(env_name) {
        return command_works(&tool).then_some(tool);
    }
    fallbacks.iter().map(OsString::from).find(|tool| command_works(tool))
}

fn command_works(tool: &OsStr) -> bool {
    StdCommand::new(tool).arg("--version").output().is_ok_and(|output| output.status.success())
}

fn gcc_sysroot(gcc: &OsStr) -> Option<PathBuf> {
    let output = StdCommand::new(gcc).arg("-print-sysroot").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let sysroot = PathBuf::from(String::from_utf8(output.stdout).ok()?.trim());
    (!sysroot.as_os_str().is_empty() && sysroot.is_dir()).then_some(sysroot)
}

fn runtime_unavailable(required: bool, reason: &str) {
    if required {
        panic!("RV64 runtime tools were required: {reason}");
    }
    eprintln!("skipping optional RV64 QEMU runtime coverage: {reason}");
    eprintln!(
        "set RISCV64_GCC and QEMU_RISCV64 to enable it, or set \
         RVV_ABI_INTEROP_REQUIRE_RUNTIME=1 to make missing tools an error"
    );
}
