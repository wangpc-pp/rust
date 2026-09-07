//! Check that RISC-V ELF objects mark functions using the vector calling convention with
//! `STO_RISCV_VARIANT_CC`. This inspects the objects directly, without linking or running them.

//@ needs-llvm-components: riscv

use run_make_support::{llvm_readobj, path, rustc, rustc_minicore};

fn main() {
    for (name, target) in
        [("rv32", "riscv32gc-unknown-linux-gnu"), ("rv64", "riscv64gc-unknown-linux-gnu")]
    {
        let minicore = format!("libminicore-{name}.rlib");
        rustc_minicore().target(target).panic("abort").output(&minicore).run();

        let object = format!("variant-cc-{name}.o");
        rustc()
            .input("lib.rs")
            .target(target)
            .panic("abort")
            .arg("-Ctarget-feature=+v")
            .arg("-Zmerge-functions=disabled")
            .extern_("minicore", path(&minicore))
            .emit(format!("obj={object}"))
            .run();

        let symbols = llvm_readobj().input(&object).symbols().run().stdout_utf8();
        for symbol in ["vector_argument", "vector_return", "tuple_argument", "tuple_return"] {
            assert_variant_cc(&symbols, symbol, true);
        }
        assert_variant_cc(&symbols, "scalar", false);
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
