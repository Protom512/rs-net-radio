use record_lib::record::hibiki;
use std::env::set_var;

fn main() {
    set_var("RS_NET_ARCHIVE_PATH", "./Temp");
    hibiki::record();

    println!("{}", common::format_forbidden_char("Fate/Test"));
}
