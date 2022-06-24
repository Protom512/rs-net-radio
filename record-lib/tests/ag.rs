use chrono::Local;

extern crate record_lib;
use record_lib::record::ag::Ag;
use std::fs::File;
#[test]
fn check_connection() {
    assert_eq!(Ag::get_html().status(), http::StatusCode::OK)
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
fn test_init() {
    let f = File::open("./ag.html");
    print!("{:#?}", f);
    let _resp = Ag::get_html();
    //assert_eq!(Ag::init())
}
