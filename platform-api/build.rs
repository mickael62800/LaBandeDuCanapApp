fn main() {
    println!("cargo:rerun-if-changed=migrations/atrium");
    println!("cargo:rerun-if-changed=migrations/nexus");
    println!("cargo:rerun-if-changed=migrations/sentinel");
}
