use test_utils::TestDir;

#[test]
fn placeholder() {
    let dir = TestDir::new("wal");
    assert!(dir.path().exists());
}
