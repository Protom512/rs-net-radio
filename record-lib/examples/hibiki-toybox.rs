use record_lib::record::hibiki;
use std::env::set_var;

fn main() {
    set_var(hibiki::RS_NET_ARCHIVE_PATH_KEY, "./Temp");
    hibiki::record();

    println!("{}", hibiki::format_forbidden_char("Fate/Test"));
}
