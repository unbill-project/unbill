fn main() {
    build_info_build::build_script().build_timestamp(chrono::Utc::now());
}
