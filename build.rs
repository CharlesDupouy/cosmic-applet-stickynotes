// SPDX-License-Identifier: MIT
// build.rs — no generated files needed; .desktop is a static resource in res/.
fn main() {
    println!("cargo:rerun-if-changed=res");
}
