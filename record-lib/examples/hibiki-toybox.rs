use record_lib::record::hibiki;
use record_lib::utils::sanitize_filename; // Add this
use std::env::set_var;

fn main() {
    set_var("RS_NET_ARCHIVE_PATH", "./Temp");
    hibiki::record();

    println!("{}", sanitize_filename("Fate/Test"));
}
