//! Programmatic surface and lowering for the RISC-V vector-crypto intrinsics.
//!
//! The regular RVV surface is compactly described by `rvv.yml`.  The
//! vector-crypto families have a small number of operations but several
//! non-uniform type grids, so keeping those grids and their LLVM mangling in
//! one module makes the correspondence with Clang's TableGen definitions
//! explicit.

#[cfg(test)]
use std::collections::BTreeSet;

use crate::adoc::Proto;
use crate::emit::{Op, Recipe};
use crate::types::{nxv, vtype_of_rust};

/// Zvkb/Zvbb/Zvbc operations in the order used by Clang's TableGen file.
pub const BASIC_FAMILIES: &[&str] = &[
    "vandn", "vbrev8", "vrev8", "vrol", "vror", "vbrev", "vclz", "vctz", "vcpop", "vwsll",
    "vclmul", "vclmulh",
];

/// Zvkg/Zvkned/Zvknha/Zvknhb/Zvksed/Zvksh operations in Clang's order.
pub const ADVANCED_FAMILIES: &[&str] = &[
    "vghsh", "vgmul", "vaesdf", "vaesdm", "vaesef", "vaesem", "vaeskf1", "vaeskf2", "vaesz",
    "vsha2ch", "vsha2cl", "vsha2ms", "vsm4k", "vsm4r", "vsm3c", "vsm3me",
];

const LMULS: &[(&str, u32, u32)] = &[
    ("mf8", 1, 8),
    ("mf4", 1, 4),
    ("mf2", 1, 2),
    ("m1", 1, 1),
    ("m2", 2, 1),
    ("m4", 4, 1),
    ("m8", 8, 1),
];

#[derive(Clone)]
struct UIntType {
    suffix: String,
    vector: String,
    scalar: String,
    lanes: u32,
}

#[derive(Clone, Copy)]
enum ScalarOperand {
    Element,
    XLen,
}

/// Expand one vector-crypto family named by an `rvv.yml` `kind:
/// vector_crypto` entry.
///
/// Basic families include exactly Clang 22's non-policy, non-overloaded base
/// and `_m` variants. Advanced families are the unmasked vector-crypto
/// operations, which use their policy operand to request tail-agnostic output.
pub fn expand_family(name: &str, doc: &str) -> Option<Vec<Proto>> {
    expand_basic_family(name, doc).or_else(|| expand_advanced_family(name, doc))
}

fn expand_basic_family(name: &str, doc: &str) -> Option<Vec<Proto>> {
    let types = unsigned_types(&[8, 16, 32, 64]);
    let protos = match name {
        "vandn" => binary(name, &types, ScalarOperand::Element, doc),
        "vrol" | "vror" => binary(name, &types, ScalarOperand::XLen, doc),
        "vbrev8" | "vrev8" | "vbrev" | "vclz" | "vctz" | "vcpop" => unary(name, &types, doc),
        "vwsll" => widening_shift_left(name, doc),
        "vclmul" | "vclmulh" => {
            let types = unsigned_types(&[64]);
            binary(name, &types, ScalarOperand::Element, doc)
        }
        _ => return None,
    };
    Some(protos)
}

fn expand_advanced_family(name: &str, doc: &str) -> Option<Vec<Proto>> {
    let u32_types = unsigned_types(&[32]);
    let protos = match name {
        "vghsh" => ternary_accumulator(name, &u32_types, doc),
        "vgmul" => binary_accumulator(name, &u32_types, doc),
        "vaesdf" | "vaesdm" | "vaesef" | "vaesem" | "vsm4r" => {
            vv_vs_matrix(name, &u32_types, true, doc)
        }
        "vaesz" => vv_vs_matrix(name, &u32_types, false, doc),
        "vaeskf1" | "vsm4k" => unary_immediate(name, &u32_types, doc),
        "vaeskf2" | "vsm3c" => binary_immediate(name, &u32_types, doc),
        "vsha2ch" | "vsha2cl" | "vsha2ms" => {
            let types = unsigned_types(&[32, 64]);
            ternary_accumulator(name, &types, doc)
        }
        "vsm3me" => binary(name, &u32_types, ScalarOperand::Element, doc)
            .into_iter()
            .filter(|p| p.short().contains("_vv_") && !p.short().ends_with("_m"))
            .collect(),
        _ => return None,
    };
    Some(protos)
}

fn unsigned_types(sews: &[u32]) -> Vec<UIntType> {
    let mut out = Vec::new();
    for &sew in sews {
        for &(lmul, numerator, denominator) in LMULS {
            let lane_numerator = 64 * numerator;
            let lane_denominator = sew * denominator;
            if lane_numerator % lane_denominator != 0 {
                continue;
            }
            let lanes = lane_numerator / lane_denominator;
            if lanes == 0 {
                continue;
            }
            out.push(UIntType {
                suffix: format!("u{sew}{lmul}"),
                vector: format!("vuint{sew}{lmul}_t"),
                scalar: format!("uint{sew}_t"),
                lanes,
            });
        }
    }
    out
}

fn proto(ret: &str, name: String, params: Vec<(String, String)>, doc: &str) -> Proto {
    Proto {
        ret: ret.to_string(),
        name: format!("__riscv_{name}"),
        params,
        section: doc.to_string(),
    }
}

fn masked(mut p: Proto, lanes: u32) -> Proto {
    p.name.push_str("_m");
    p.params
        .insert(0, (format!("vbool{}_t", 64 / lanes), "mask".to_string()));
    p
}

fn unary(name: &str, types: &[UIntType], doc: &str) -> Vec<Proto> {
    let base: Vec<_> = types
        .iter()
        .map(|ty| {
            proto(
                &ty.vector,
                format!("{name}_v_{}", ty.suffix),
                vec![
                    (ty.vector.clone(), "vs2".to_string()),
                    ("size_t".to_string(), "vl".to_string()),
                ],
                doc,
            )
        })
        .collect();
    with_masked(base, types.iter().map(|ty| ty.lanes))
}

fn binary(name: &str, types: &[UIntType], scalar_operand: ScalarOperand, doc: &str) -> Vec<Proto> {
    let mut base = Vec::new();
    let mut lanes = Vec::new();
    for ty in types {
        base.push(proto(
            &ty.vector,
            format!("{name}_vv_{}", ty.suffix),
            vec![
                (ty.vector.clone(), "vs2".to_string()),
                (ty.vector.clone(), "vs1".to_string()),
                ("size_t".to_string(), "vl".to_string()),
            ],
            doc,
        ));
        lanes.push(ty.lanes);
        let scalar = match scalar_operand {
            ScalarOperand::Element => ty.scalar.clone(),
            ScalarOperand::XLen => "size_t".to_string(),
        };
        base.push(proto(
            &ty.vector,
            format!("{name}_vx_{}", ty.suffix),
            vec![
                (ty.vector.clone(), "vs2".to_string()),
                (scalar, "rs1".to_string()),
                ("size_t".to_string(), "vl".to_string()),
            ],
            doc,
        ));
        lanes.push(ty.lanes);
    }
    with_masked(base, lanes)
}

fn binary_accumulator(name: &str, types: &[UIntType], doc: &str) -> Vec<Proto> {
    types
        .iter()
        .map(|ty| {
            proto(
                &ty.vector,
                format!("{name}_vv_{}", ty.suffix),
                vec![
                    (ty.vector.clone(), "vd".to_string()),
                    (ty.vector.clone(), "vs2".to_string()),
                    ("size_t".to_string(), "vl".to_string()),
                ],
                doc,
            )
        })
        .collect()
}

fn ternary_accumulator(name: &str, types: &[UIntType], doc: &str) -> Vec<Proto> {
    types
        .iter()
        .map(|ty| {
            proto(
                &ty.vector,
                format!("{name}_vv_{}", ty.suffix),
                vec![
                    (ty.vector.clone(), "vd".to_string()),
                    (ty.vector.clone(), "vs2".to_string()),
                    (ty.vector.clone(), "vs1".to_string()),
                    ("size_t".to_string(), "vl".to_string()),
                ],
                doc,
            )
        })
        .collect()
}

/// Generate the vector-vector form and the vector-scalar-element-group form.
///
/// Clang deliberately uses a triangular two-LMUL matrix for `.vs`: the source
/// group is one of mf2/m1/m2/m4 and is no wider than the destination group.
/// The source LMUL is written first in the public C/Rust suffix.
fn vv_vs_matrix(name: &str, types: &[UIntType], has_vv: bool, doc: &str) -> Vec<Proto> {
    let mut out = Vec::new();
    for (source_index, source) in types.iter().enumerate() {
        if has_vv {
            out.push(proto(
                &source.vector,
                format!("{name}_vv_{}", source.suffix),
                vec![
                    (source.vector.clone(), "vd".to_string()),
                    (source.vector.clone(), "vs2".to_string()),
                    ("size_t".to_string(), "vl".to_string()),
                ],
                doc,
            ));
        }
        // The fixed scalar element-group source never uses LMUL=m8.
        if source.suffix.ends_with("m8") {
            continue;
        }
        for destination in &types[source_index..] {
            out.push(proto(
                &destination.vector,
                format!("{name}_vs_{}_{}", source.suffix, destination.suffix),
                vec![
                    (destination.vector.clone(), "vd".to_string()),
                    (source.vector.clone(), "vs2".to_string()),
                    ("size_t".to_string(), "vl".to_string()),
                ],
                doc,
            ));
        }
    }
    out
}

fn unary_immediate(name: &str, types: &[UIntType], doc: &str) -> Vec<Proto> {
    types
        .iter()
        .map(|ty| {
            proto(
                &ty.vector,
                format!("{name}_vi_{}", ty.suffix),
                vec![
                    (ty.vector.clone(), "vs2".to_string()),
                    ("size_t".to_string(), "uimm".to_string()),
                    ("size_t".to_string(), "vl".to_string()),
                ],
                doc,
            )
        })
        .collect()
}

fn binary_immediate(name: &str, types: &[UIntType], doc: &str) -> Vec<Proto> {
    types
        .iter()
        .map(|ty| {
            proto(
                &ty.vector,
                format!("{name}_vi_{}", ty.suffix),
                vec![
                    (ty.vector.clone(), "vd".to_string()),
                    (ty.vector.clone(), "vs2".to_string()),
                    ("size_t".to_string(), "uimm".to_string()),
                    ("size_t".to_string(), "vl".to_string()),
                ],
                doc,
            )
        })
        .collect()
}

fn widening_shift_left(name: &str, doc: &str) -> Vec<Proto> {
    let mut base = Vec::new();
    let mut lanes = Vec::new();
    for dst in unsigned_types(&[16, 32, 64]) {
        let src_sew = dst
            .suffix
            .strip_prefix('u')
            .and_then(|s| s.split_once('m'))
            .and_then(|(sew, _)| sew.parse::<u32>().ok())
            .expect("generated unsigned suffix")
            / 2;
        let src = unsigned_types(&[src_sew])
            .into_iter()
            .find(|src| src.lanes == dst.lanes)
            .expect("a widening source with the destination lane count");
        base.push(proto(
            &dst.vector,
            format!("{name}_vv_{}", dst.suffix),
            vec![
                (src.vector.clone(), "vs2".to_string()),
                (src.vector.clone(), "vs1".to_string()),
                ("size_t".to_string(), "vl".to_string()),
            ],
            doc,
        ));
        lanes.push(dst.lanes);
        base.push(proto(
            &dst.vector,
            format!("{name}_vx_{}", dst.suffix),
            vec![
                (src.vector, "vs2".to_string()),
                ("size_t".to_string(), "rs1".to_string()),
                ("size_t".to_string(), "vl".to_string()),
            ],
            doc,
        ));
        lanes.push(dst.lanes);
    }
    with_masked(base, lanes)
}

fn with_masked(mut base: Vec<Proto>, lanes: impl IntoIterator<Item = u32>) -> Vec<Proto> {
    let masked = base
        .iter()
        .cloned()
        .zip(lanes)
        .map(|(p, lanes)| masked(p, lanes))
        .collect::<Vec<_>>();
    base.extend(masked);
    base
}

/// Whether `short` names one of the basic Zvkb/Zvbb/Zvbc operations.
pub fn is_basic_crypto(short: &str) -> bool {
    short
        .split('_')
        .next()
        .is_some_and(|op| BASIC_FAMILIES.contains(&op))
}

/// The complete Clang 22 non-policy/non-overloaded basic-crypto name set.
#[cfg(test)]
pub fn expected_names() -> BTreeSet<String> {
    BASIC_FAMILIES
        .iter()
        .flat_map(|name| expand_basic_family(name, "").expect("known basic family"))
        .map(|p| p.name)
        .collect()
}

/// Build the exact LLVM intrinsic recipe for a generated vector-crypto API.
/// `iXLEN` is expanded by the emitter to `i32`/`i64` in target-specific link
/// names; this matters for rotate and widening-shift scalar forms, where both
/// the scalar shift amount and `vl` are iXLEN.
pub fn lowering_recipe(p: &Proto) -> Option<Recipe> {
    lowering_basic_recipe(p).or_else(|| lowering_advanced_recipe(p))
}

/// Extra target features that depend on a concrete vector-crypto prototype.
/// Advanced families use this for element-group-width `zvl*` requirements.
pub fn additional_features(p: &Proto) -> Vec<&'static str> {
    let operation = p.short().split('_').next().unwrap_or("");
    let mut features = Vec::new();

    // Zvknhb contains the SHA-256 operations too, but Clang exposes the u32
    // overloads with Zvknha and reserves Zvknhb for the u64 overloads.
    let egw = match operation {
        "vsha2ch" | "vsha2cl" | "vsha2ms" => {
            let sew = vtype_of_rust(&p.ret).map(|ty| ty.sew).unwrap_or(32);
            features.push(if sew == 64 { "zvknhb" } else { "zvknha" });
            4 * sew
        }
        // These instructions operate on one 128-bit element group.  LLVM 22
        // does not diagnose undersized Zvkg groups, but the architecture gives
        // vghsh/vgmul the same EGW requirement as AES and SM4.
        "vghsh" | "vgmul" | "vaesdf" | "vaesdm" | "vaesef" | "vaesem" | "vaeskf1" | "vaeskf2"
        | "vaesz" | "vsm4k" | "vsm4r" => 128,
        // SM3 operates on one 256-bit element group.
        "vsm3c" | "vsm3me" => 256,
        _ => return features,
    };

    let minimum_vlen = std::iter::once(&p.ret)
        .chain(p.params.iter().map(|(ty, _)| ty))
        .filter_map(|ty| vtype_of_rust(ty))
        .map(|ty| (egw * 64).div_ceil(ty.n * ty.sew))
        .max()
        .unwrap_or(0);
    if minimum_vlen > 256 {
        features.push("zvl512b");
    } else if minimum_vlen > 128 {
        features.push("zvl256b");
    }
    features
}

/// Maximum accepted value for a vector-crypto immediate operand.
/// The basic Zvkb/Zvbb/Zvbc surface has no public immediate parameters.
pub fn immediate_max(short: &str, param: &str) -> Option<usize> {
    let operation = short.split('_').next()?;
    (matches!(param, "uimm" | "UIMM")
        && matches!(operation, "vaeskf1" | "vaeskf2" | "vsm4k" | "vsm3c"))
    .then_some(31)
}

fn lowering_basic_recipe(p: &Proto) -> Option<Recipe> {
    if !is_basic_crypto(p.short()) {
        return None;
    }
    let parts: Vec<_> = p.short().split('_').collect();
    let masked = match parts.as_slice() {
        [_, _, _] => false,
        [_, _, _, "m"] => true,
        // Leave policy variants to the policy manifest/recipe provider.
        _ => return None,
    };
    let operation = parts[0];
    let form = parts[1];
    let ret = nxv(vtype_of_rust(&p.ret)?);
    let ir_operation = if operation == "vcpop" {
        "vcpopv"
    } else {
        operation
    };

    let source_vectors: Vec<_> = p
        .params
        .iter()
        .filter_map(|(ty, name)| {
            (!matches!(name.as_str(), "mask" | "vl"))
                .then(|| vtype_of_rust(ty).map(nxv))
                .flatten()
        })
        .collect();
    let overloads = match (operation, form) {
        ("vwsll", "vv") => format!(".{ret}.{}.{}", source_vectors[0], source_vectors[1]),
        ("vwsll", "vx") => format!(".{ret}.{}.iXLEN", source_vectors[0]),
        ("vrol" | "vror", "vx") => format!(".{ret}.iXLEN"),
        ("vandn", "vx") => {
            let sew = vtype_of_rust(&p.ret)?.sew;
            format!(".{ret}.i{sew}")
        }
        ("vclmul" | "vclmulh", "vx") => format!(".{ret}.i64"),
        (_, "vv") => format!(".{ret}.{}", source_vectors[1]),
        (_, "v") => format!(".{ret}"),
        _ => return None,
    };
    let mask_component = if masked { ".mask" } else { "" };
    let llvm = format!("llvm.riscv.{ir_operation}{mask_component}{overloads}.iXLEN");

    let ops = if form == "v" {
        if masked {
            vec![
                Op::Poison,
                Op::Param(1),
                Op::Param(0),
                Op::Param(2),
                Op::xlen(3),
            ]
        } else {
            vec![Op::Poison, Op::Param(0), Op::Param(1)]
        }
    } else if masked {
        vec![
            Op::Poison,
            Op::Param(1),
            Op::Param(2),
            Op::Param(0),
            Op::Param(3),
            Op::xlen(3),
        ]
    } else {
        vec![Op::Poison, Op::Param(0), Op::Param(1), Op::Param(2)]
    };
    Some(Recipe { llvm, ops })
}

fn lowering_advanced_recipe(p: &Proto) -> Option<Recipe> {
    let parts: Vec<_> = p.short().split('_').collect();
    let operation = *parts.first()?;
    if !ADVANCED_FAMILIES.contains(&operation) {
        return None;
    }
    // Policy variants have additional suffix components and are provided by
    // the policy manifest rather than this non-policy recipe provider.
    let form = *parts.get(1)?;
    if (form == "vs" && parts.len() != 4) || (form != "vs" && parts.len() != 3) {
        return None;
    }

    let ret = nxv(vtype_of_rust(&p.ret)?);
    let source = || {
        p.params
            .iter()
            .find(|(_, name)| name == "vs2")
            .and_then(|(ty, _)| vtype_of_rust(ty))
            .map(nxv)
    };
    let (llvm, ops) = match (operation, form) {
        ("vghsh", "vv") => (
            format!("llvm.riscv.vghsh.{ret}.{ret}.iXLEN"),
            vec![
                Op::Param(0),
                Op::Param(1),
                Op::Param(2),
                Op::Param(3),
                Op::xlen(3),
            ],
        ),
        ("vgmul", "vv") => (
            format!("llvm.riscv.vgmul.vv.{ret}.iXLEN"),
            vec![Op::Param(0), Op::Param(1), Op::Param(2), Op::xlen(3)],
        ),
        ("vaesdf" | "vaesdm" | "vaesef" | "vaesem" | "vsm4r", "vv") => (
            format!("llvm.riscv.{operation}.vv.{ret}.iXLEN"),
            vec![Op::Param(0), Op::Param(1), Op::Param(2), Op::xlen(3)],
        ),
        ("vaesdf" | "vaesdm" | "vaesef" | "vaesem" | "vaesz" | "vsm4r", "vs") => (
            format!("llvm.riscv.{operation}.vs.{ret}.{}.iXLEN", source()?),
            vec![Op::Param(0), Op::Param(1), Op::Param(2), Op::xlen(3)],
        ),
        ("vaeskf1" | "vsm4k", "vi") => (
            format!("llvm.riscv.{operation}.{ret}.iXLEN.iXLEN"),
            vec![Op::Poison, Op::Param(0), Op::Param(1), Op::Param(2)],
        ),
        ("vaeskf2", "vi") => (
            format!("llvm.riscv.vaeskf2.{ret}.iXLEN.iXLEN"),
            vec![
                Op::Param(0),
                Op::Param(1),
                Op::Param(2),
                Op::Param(3),
                Op::xlen(3),
            ],
        ),
        ("vsha2ch" | "vsha2cl" | "vsha2ms", "vv") => (
            format!("llvm.riscv.{operation}.{ret}.{ret}.iXLEN"),
            vec![
                Op::Param(0),
                Op::Param(1),
                Op::Param(2),
                Op::Param(3),
                Op::xlen(3),
            ],
        ),
        ("vsm3c", "vi") => (
            format!("llvm.riscv.vsm3c.{ret}.iXLEN.iXLEN"),
            vec![
                Op::Param(0),
                Op::Param(1),
                Op::Param(2),
                Op::Param(3),
                Op::xlen(3),
            ],
        ),
        ("vsm3me", "vv") => (
            format!("llvm.riscv.vsm3me.{ret}.{ret}.iXLEN"),
            vec![Op::Poison, Op::Param(0), Op::Param(1), Op::Param(2)],
        ),
        _ => return None,
    };
    Some(Recipe { llvm, ops })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::LazyLock;

    use super::*;
    use crate::adoc::parse_proto;

    fn clang_test_source(name: &str) -> &'static str {
        let path = format!(
            "{}/testdata/vector_crypto/{name}.c",
            env!("CARGO_MANIFEST_DIR")
        );
        Box::leak(
            std::fs::read_to_string(path)
                .expect("vector crypto fixture")
                .into_boxed_str(),
        )
    }

    static CLANG_BASIC_TESTS: LazyLock<Vec<(&'static str, &'static str)>> = LazyLock::new(|| {
        [
            "vandn", "vbrev8", "vrev8", "vrol", "vror", "vbrev", "vclz", "vctz", "vcpop", "vwsll",
            "vclmul", "vclmulh",
        ]
        .into_iter()
        .map(|name| (name, clang_test_source(name)))
        .collect()
    });

    static CLANG_ADVANCED_TESTS: LazyLock<Vec<(&'static str, &'static str)>> =
        LazyLock::new(|| {
            [
                "vghsh", "vgmul", "vaesdf", "vaesdm", "vaesef", "vaesem", "vaeskf1", "vaeskf2",
                "vaesz", "vsha2ch", "vsha2cl", "vsha2ms", "vsm4k", "vsm4r", "vsm3c", "vsm3me",
            ]
            .into_iter()
            .map(|name| (name, clang_test_source(name)))
            .collect()
        });

    fn clang_prototypes(source: &str) -> Vec<Proto> {
        source
            .lines()
            .filter_map(|line| {
                let declaration = line.trim().strip_suffix(" {")?;
                let (ret, rest) = declaration.split_once(" test_")?;
                parse_proto(&format!("{ret} __riscv_{rest}"), "")
            })
            .collect()
    }

    fn signature(p: &Proto) -> (&str, Vec<(&str, &str)>) {
        (
            &p.ret,
            p.params
                .iter()
                .map(|(ty, name)| (ty.as_str(), name.as_str()))
                .collect(),
        )
    }

    fn immediate_position(name: &str) -> Option<usize> {
        let operation = name.trim_start_matches("__riscv_").split('_').next()?;
        match operation {
            "vaeskf1" | "vsm4k" => Some(1),
            "vaeskf2" | "vsm3c" => Some(2),
            _ => None,
        }
    }

    /// Clang's codegen wrappers hard-code immediate zero, so their wrapper
    /// signatures omit the public `size_t uimm` argument. Restore that one
    /// argument before comparing the public surface.
    fn clang_advanced_prototypes(source: &str) -> Vec<Proto> {
        clang_prototypes(source)
            .into_iter()
            .map(|mut p| {
                if let Some(position) = immediate_position(&p.name) {
                    p.params
                        .insert(position, ("size_t".to_string(), "uimm".to_string()));
                }
                p
            })
            .collect()
    }

    fn clang_llvm_names(source: &str) -> BTreeMap<String, String> {
        let mut current = None;
        let mut out = BTreeMap::new();
        for line in source.lines() {
            if line.contains("-LABEL:") {
                current = line.split_once("@test_").map(|(_, tail)| {
                    let name = tail
                        .split(|c: char| c == '(' || c.is_whitespace())
                        .next()
                        .expect("Clang RVV label has a function name");
                    format!("__riscv_{name}")
                });
            }
            if !line.contains(" = call ") {
                continue;
            }
            let Some(name) = current.take() else {
                // Ignore intrinsic declarations following each wrapper.
                continue;
            };
            let llvm = line
                .split_once("@llvm.riscv.")
                .and_then(|(_, tail)| tail.split_once('('))
                .map(|(name, _)| format!("llvm.riscv.{name}"))
                .expect("Clang RVV check call has an LLVM intrinsic");
            out.insert(name, llvm);
        }
        out
    }

    fn clang_recipes(source: &str) -> BTreeMap<String, Recipe> {
        let mut current_name = None;
        let mut current_params = BTreeMap::new();
        let mut out = BTreeMap::new();
        for line in source.lines() {
            if line.contains("-LABEL:") {
                current_name = line.split_once("@test_").map(|(_, tail)| {
                    let name = tail
                        .split(|c: char| c == '(' || c.is_whitespace())
                        .next()
                        .expect("Clang RVV label has a function name");
                    format!("__riscv_{name}")
                });
                current_params.clear();
            } else if line.contains("-SAME:") {
                let Some((_, args)) = line.split_once('(') else {
                    continue;
                };
                for (index, arg) in args.split(',').enumerate() {
                    if let Some((_, tail)) = arg.split_once("[[") {
                        let name = tail
                            .split(|c: char| c == ':' || c == ']')
                            .next()
                            .unwrap()
                            .to_ascii_lowercase();
                        current_params.insert(name, index);
                    }
                }
            }
            if !line.contains(" = call ") {
                continue;
            }
            let Some(name) = current_name.take() else {
                continue;
            };
            let (_, call) = line
                .split_once("@llvm.riscv.")
                .expect("Clang RVV check call has an intrinsic");
            let (llvm, args) = call
                .split_once('(')
                .expect("Clang RVV intrinsic call has operands");
            let ops = args
                .trim_end_matches(')')
                .split(',')
                .map(|arg| {
                    if arg.contains(" poison") || arg.contains(" undef") {
                        return Op::Poison;
                    }
                    if let Some((_, tail)) = arg.split_once("[[") {
                        let operand = tail
                            .split(|c: char| c == ':' || c == ']')
                            .next()
                            .unwrap()
                            .to_ascii_lowercase();
                        return Op::Param(current_params[&operand]);
                    }
                    let value = arg
                        .split_whitespace()
                        .last()
                        .and_then(|value| value.parse().ok())
                        .expect("literal Clang RVV operand");
                    let width = arg
                        .split_whitespace()
                        .next()
                        .and_then(|ty| ty.strip_prefix('i'))
                        .and_then(|width| width.parse().ok())
                        .expect("literal Clang RVV operand type");
                    if width == 64 {
                        Op::xlen(value)
                    } else {
                        Op::fixed(value, width)
                    }
                })
                .collect();
            out.insert(
                name,
                Recipe {
                    llvm: format!("llvm.riscv.{llvm}"),
                    ops,
                },
            );
        }
        out
    }

    fn clang_advanced_recipes(source: &str) -> BTreeMap<String, Recipe> {
        clang_recipes(source)
            .into_iter()
            .map(|(name, mut recipe)| {
                if let Some(position) = immediate_position(&name) {
                    for operand in &mut recipe.ops {
                        if let Op::Param(index) = operand
                            && *index >= position
                        {
                            *index += 1;
                        }
                    }
                    let immediate = recipe
                        .ops
                        .iter_mut()
                        .find(|operand| matches!(operand, Op::Const { value: 0, .. }))
                        .expect("Clang crypto wrapper has an immediate zero");
                    *immediate = Op::Param(position);
                }
                (name, recipe)
            })
            .collect()
    }

    #[test]
    fn basic_surface_matches_vendored_clang_22_names_and_signatures() {
        let clang: BTreeMap<_, _> = CLANG_BASIC_TESTS
            .iter()
            .flat_map(|(_, source)| clang_prototypes(source))
            .map(|p| (p.name.clone(), p))
            .collect();
        let generated: BTreeMap<_, _> = BASIC_FAMILIES
            .iter()
            .flat_map(|name| expand_basic_family(name, "").unwrap())
            .map(|p| (p.name.clone(), p))
            .collect();

        assert_eq!(generated.len(), 620);
        assert_eq!(
            generated.keys().collect::<Vec<_>>(),
            clang.keys().collect::<Vec<_>>()
        );
        for (name, generated) in &generated {
            assert_eq!(signature(generated), signature(&clang[name]), "{name}");
        }
        assert_eq!(expected_names(), generated.into_keys().collect());
    }

    #[test]
    fn basic_family_counts_match_vendored_clang_22() {
        let expected = [
            ("vandn", 88),
            ("vbrev8", 44),
            ("vrev8", 44),
            ("vrol", 88),
            ("vror", 88),
            ("vbrev", 44),
            ("vclz", 44),
            ("vctz", 44),
            ("vcpop", 44),
            ("vwsll", 60),
            ("vclmul", 16),
            ("vclmulh", 16),
        ];
        for (name, count) in expected {
            assert_eq!(
                expand_basic_family(name, "").unwrap().len(),
                count,
                "{name}"
            );
        }
    }

    #[test]
    fn basic_lowering_names_match_vendored_clang_22() {
        let clang: BTreeMap<_, _> = CLANG_BASIC_TESTS
            .iter()
            .flat_map(|(_, source)| clang_llvm_names(source))
            .collect();
        assert_eq!(clang.len(), 620);

        for family in BASIC_FAMILIES {
            for proto in expand_basic_family(family, "").unwrap() {
                let recipe = lowering_basic_recipe(&proto).expect("basic crypto recipe");
                assert_eq!(
                    recipe.llvm.replace("iXLEN", "i64"),
                    clang[&proto.name],
                    "{}",
                    proto.name
                );
            }
        }
    }

    #[test]
    fn basic_lowering_recipes_match_vendored_clang_22() {
        let mut clang: BTreeMap<_, _> = CLANG_BASIC_TESTS
            .iter()
            .flat_map(|(_, source)| clang_recipes(source))
            .collect();
        for recipe in clang.values_mut() {
            for operand in &mut recipe.ops {
                if let Op::Const { ty, .. } = operand {
                    *ty = crate::emit::IntTy::XLen;
                }
            }
        }
        assert_eq!(clang.len(), 620);

        for family in BASIC_FAMILIES {
            for proto in expand_basic_family(family, "").unwrap() {
                let mut recipe = lowering_basic_recipe(&proto).expect("basic crypto recipe");
                recipe.llvm = recipe.llvm.replace("iXLEN", "i64");
                assert_eq!(recipe, clang[&proto.name], "{}", proto.name);
            }
        }
    }

    #[test]
    fn advanced_surface_matches_vendored_clang_22_names_and_signatures() {
        let clang: BTreeMap<_, _> = CLANG_ADVANCED_TESTS
            .iter()
            .flat_map(|(_, source)| clang_advanced_prototypes(source))
            .map(|p| (p.name.clone(), p))
            .collect();
        let generated: BTreeMap<_, _> = ADVANCED_FAMILIES
            .iter()
            .flat_map(|name| expand_advanced_family(name, "").unwrap())
            .map(|p| (p.name.clone(), p))
            .collect();

        assert_eq!(generated.len(), 171);
        assert_eq!(
            generated.keys().collect::<Vec<_>>(),
            clang.keys().collect::<Vec<_>>()
        );
        for (name, generated) in &generated {
            assert_eq!(signature(generated), signature(&clang[name]), "{name}");
        }
    }

    #[test]
    fn advanced_family_counts_match_vendored_clang_22() {
        let expected = [
            ("vghsh", 5),
            ("vgmul", 5),
            ("vaesdf", 19),
            ("vaesdm", 19),
            ("vaesef", 19),
            ("vaesem", 19),
            ("vaeskf1", 5),
            ("vaeskf2", 5),
            ("vaesz", 14),
            ("vsha2ch", 9),
            ("vsha2cl", 9),
            ("vsha2ms", 9),
            ("vsm4k", 5),
            ("vsm4r", 19),
            ("vsm3c", 5),
            ("vsm3me", 5),
        ];
        for (name, count) in expected {
            assert_eq!(
                expand_advanced_family(name, "").unwrap().len(),
                count,
                "{name}"
            );
        }
    }

    #[test]
    fn advanced_lowering_recipes_match_vendored_clang_22() {
        let mut clang: BTreeMap<_, _> = CLANG_ADVANCED_TESTS
            .iter()
            .flat_map(|(_, source)| clang_advanced_recipes(source))
            .collect();
        for recipe in clang.values_mut() {
            for operand in &mut recipe.ops {
                if let Op::Const { ty, .. } = operand {
                    *ty = crate::emit::IntTy::XLen;
                }
            }
        }
        assert_eq!(clang.len(), 171);

        for family in ADVANCED_FAMILIES {
            for proto in expand_advanced_family(family, "").unwrap() {
                let mut recipe = lowering_advanced_recipe(&proto).expect("advanced crypto recipe");
                recipe.llvm = recipe.llvm.replace("iXLEN", "i64");
                assert_eq!(recipe, clang[&proto.name], "{}", proto.name);
            }
        }
    }

    #[test]
    fn advanced_immediates_are_uimm5_const_generics() {
        let immediate_protos: Vec<_> = ["vaeskf1", "vaeskf2", "vsm4k", "vsm3c"]
            .into_iter()
            .flat_map(|name| expand_advanced_family(name, "").unwrap())
            .collect();
        assert_eq!(immediate_protos.len(), 20);
        for proto in immediate_protos {
            assert!(
                proto
                    .params
                    .contains(&("size_t".to_string(), "uimm".to_string()))
            );
            assert_eq!(immediate_max(proto.short(), "uimm"), Some(31));
            assert_eq!(immediate_max(proto.short(), "UIMM"), Some(31));
        }
        assert_eq!(immediate_max("vaesdf_vv_u32m1", "UIMM"), None);
    }

    #[test]
    fn advanced_egw_and_sha_feature_boundaries_are_explicit() {
        let find = |family: &str, name: &str| {
            expand_advanced_family(family, "")
                .unwrap()
                .into_iter()
                .find(|p| p.short() == name)
                .unwrap()
        };

        assert_eq!(
            additional_features(&find("vaesdf", "vaesdf_vv_u32mf2")),
            ["zvl256b"]
        );
        assert!(additional_features(&find("vaesdf", "vaesdf_vv_u32m1")).is_empty());
        assert_eq!(
            additional_features(&find("vaesdf", "vaesdf_vs_u32mf2_u32m8")),
            ["zvl256b"]
        );
        assert_eq!(
            additional_features(&find("vsha2ch", "vsha2ch_vv_u32mf2")),
            ["zvknha", "zvl256b"]
        );
        assert_eq!(
            additional_features(&find("vsha2ch", "vsha2ch_vv_u32m1")),
            ["zvknha"]
        );
        assert_eq!(
            additional_features(&find("vsha2ch", "vsha2ch_vv_u64m1")),
            ["zvknhb", "zvl256b"]
        );
        assert_eq!(
            additional_features(&find("vsha2ch", "vsha2ch_vv_u64m2")),
            ["zvknhb"]
        );
        assert_eq!(
            additional_features(&find("vsm3c", "vsm3c_vi_u32mf2")),
            ["zvl512b"]
        );
        assert_eq!(
            additional_features(&find("vsm3c", "vsm3c_vi_u32m1")),
            ["zvl256b"]
        );
        assert!(additional_features(&find("vsm3c", "vsm3c_vi_u32m2")).is_empty());
    }

    #[test]
    fn policy_names_are_not_claimed_by_the_non_policy_recipe_provider() {
        let mut p = expand_basic_family("vandn", "").unwrap().remove(0);
        p.name.push_str("_tu");
        assert!(lowering_basic_recipe(&p).is_none());
    }
}
