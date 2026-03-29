use test_utils::TestDir;

#[test]
fn placeholder() {
    let dir = TestDir::new("parser");
    assert!(dir.path().exists());
}
