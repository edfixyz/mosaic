const VERSION_STRING: &str = "MOSAIC 2025.10 MIDEN 0.11";

pub fn assert_version(schema: &str) {
    assert_eq!(schema, VERSION_STRING);
}