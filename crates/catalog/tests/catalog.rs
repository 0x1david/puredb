use test_utils::TestDir;

#[test]
fn placeholder() {
    let dir = TestDir::new("catalog");
    assert!(dir.path().exists());
}
