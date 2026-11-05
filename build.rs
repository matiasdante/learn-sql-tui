// `include_dir!` no avisa a cargo cuando se agregan archivos nuevos a `content/`:
// con esto, cualquier cambio en el contenido recompila el binario.
fn main() {
    println!("cargo:rerun-if-changed=content");
}
