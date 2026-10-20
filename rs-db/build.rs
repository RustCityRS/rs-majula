// `sqlx::migrate!()` embeds the migrations at compile time but doesn't watch
// the folder: rebuild when a migration is added or changed, or a build could
// miss one the database already has.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
