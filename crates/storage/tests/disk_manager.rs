use test_utils::TestDir;

#[test]
fn test_dir_provides_isolated_path() {
    let dir = TestDir::new("example");
    let db_file = dir.file("test.db");

    // Each test gets its own temp directory — no collisions.
    assert!(dir.path().exists());
    assert!(!db_file.exists()); // nothing written yet

    std::fs::write(&db_file, b"hello").unwrap();
    assert!(db_file.exists());

    // dir is dropped at end of test, cleaning up automatically.
}
