#[path = "src/revision.rs"]
mod revision;

fn main() {
    revision::compile_target_revision();
}
