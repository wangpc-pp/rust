#![deny(warnings)]

#[cfg(not(target_feature = "v"))]
compile_error!("the riscv64a23 baseline must enable the V extension");

fn reference(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }

    haystack.as_bytes().windows(needle.len()).any(|window| window == needle.as_bytes())
}

#[inline(never)]
fn check(haystack: &str, needle: &str) {
    assert_eq!(
        haystack.contains(needle),
        reference(haystack, needle),
        "haystack={haystack:?}, needle={needle:?}"
    );
}

fn main() {
    const SOURCE: &str = "0123456789abcdefghijklmnopqrstuvw";
    const FILL: &str = concat!(
        "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
    );

    // Cover empty and one-byte needles, every length handled by the SIMD path, and the first
    // length handled by the two-way searcher. Exercise the naive-search boundary, vector tails,
    // and several full vector blocks at every possible match offset.
    for needle_len in 0usize..=33 {
        let needle = &SOURCE[..needle_len];
        let load_boundary = needle_len.saturating_add(15);
        for haystack_len in [
            needle_len.saturating_sub(1),
            needle_len,
            load_boundary.saturating_sub(1),
            load_boundary,
            load_boundary + 1,
            needle_len + 64,
        ] {
            check(&FILL[..haystack_len], needle);

            if needle_len <= haystack_len {
                for offset in 0..=haystack_len - needle_len {
                    let mut haystack = FILL[..haystack_len].to_owned();
                    haystack.replace_range(offset..offset + needle_len, needle);
                    check(&haystack, needle);

                    if needle_len != 0 {
                        haystack.replace_range(offset..offset + 1, "!");
                        check(&haystack, needle);
                    }
                }
            }
        }
    }

    // Exercise the unrolled loop with matches and near misses in each vector-sized quarter.
    for offset in 0..96 {
        let mut haystack = FILL.to_owned();
        haystack.replace_range(offset..offset + 5, "abcde");
        check(&haystack, "abcde");
        check(&haystack, "abcdf");
    }

    // Repeated probe bytes make the SIMD prefilter fall back or produce many candidates.
    for (haystack, needle) in [
        ("zzzzzaaaaazzzzz", "aaaa"),
        ("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaab", "aaaab"),
        ("ababababababababababababababababababababababababababac", "ababac"),
    ] {
        check(haystack, needle);
    }

    // The implementation searches bytes, but public results must retain UTF-8 semantics.
    let unicode = "é中🦀é中🦀--needle--🦀中é";
    for needle in ["", "é", "中🦀", "🦀--needle", "needle--🦀", "中éx"] {
        check(unicode, needle);
    }
}
