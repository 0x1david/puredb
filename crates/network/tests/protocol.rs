use test_utils::TestDir;

#[test]
fn placeholder() {
    let dir = TestDir::new("protocol");
    assert!(dir.path().exists());
}
