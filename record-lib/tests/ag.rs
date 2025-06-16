use chrono::{Duration, Local, TimeZone}; // Added TimeZone for and_hms_opt
use std::fs; // Added mockito

extern crate record_lib;
use record_lib::record::ag::{get_html_from_url, Ag}; // Added get_html_from_url

#[test]
fn check_connection_mocked() {
    // For mockito 0.31, mocking is global. The path should be absolute.
    let mock_path = "/test_get_html";
    let _m = mockito::mock("GET", mock_path)
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body("mocked html content")
        .create();

    let full_mock_url = format!("{}{}", mockito::server_url(), mock_path);

    match get_html_from_url(&full_mock_url) {
        Ok(response) => {
            assert_eq!(response.status(), http::StatusCode::OK);
            assert_eq!(response.text().unwrap(), "mocked html content");
        }
        Err(e) => panic!("get_html_from_url failed: {}", e),
    }
    // _m.assert(); // .assert() is available on the mock guard in 0.31
}

#[test]
fn check_new() {
    let title = "hoge";
    let st = Local::now();
    let et = Local::now();
    let expect = Ag {
        title: title.to_string(),
        start_datetime: st,
        end_datetime: et,
    };

    assert_eq!(Ag::new(title, &st, &et), expect);
}
#[test]
fn test_init_with_local_html() {
    // Renamed for clarity
    // Read the content of ag.html
    let html_content = fs::read_to_string("tests/ag.html")
        .expect("Should have been able to read the file tests/ag.html");

    // Parse the local HTML content
    let programs = Ag::html_parse(&html_content);

    // Add assertions
    assert!(
        !programs.is_empty(),
        "Should parse some programs from ag.html"
    );

    // Find the program "A&G ARTIST ZONE樋口楓のTHE CATCH" and assert its details
    let target_title = "A&G ARTIST ZONE樋口楓のTHE CATCH";
    let program_to_test = programs.iter().find(|p| p.title == target_title);

    if let Some(program) = program_to_test {
        // Create expected start and end times for "6:00 – 7:00" on the current day
        let local_date = Local::now().date_naive();
        let expected_start_datetime = Local
            .from_local_datetime(&local_date.and_hms_opt(6, 0, 0).unwrap())
            .unwrap();
        let expected_end_datetime = Local
            .from_local_datetime(&local_date.and_hms_opt(7, 0, 0).unwrap())
            .unwrap();

        assert_eq!(
            program.start_datetime, expected_start_datetime,
            "Start time mismatch for {}",
            target_title
        );
        assert_eq!(
            program.end_datetime, expected_end_datetime,
            "End time mismatch for {}",
            target_title
        );
    } else {
        panic!(
            "Program '{}' not found in parsed HTML. Parsed programs: {:?}",
            target_title, programs
        );
    }
}
#[test]
fn fail_record() {
    // failes
    std::env::set_var("RS_NET_ARCHIVE_PATH", "./Temp");
    let fakerecord = Ag {
        title: "hoge".to_string(),
        start_datetime: Local::now(),
        end_datetime: Local::now() + Duration::seconds(5),
    };
    assert!(fakerecord.record().is_err())
}
