use record_lib::utils::sanitize_filename;

#[test]
fn test_sanitize_filename_removes_path_traversal() {
    let malicious_input = "../../../../../../etc/passwd";
    let expected_output = "............etcpasswd";
    assert_eq!(sanitize_filename(malicious_input), expected_output);
}
