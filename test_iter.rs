use regex::Regex;
fn main() {
    let v: Vec<(Regex, usize)> = vec![(Regex::new("a").unwrap(), 0)];
    let rv = &v;
    for (re, group) in rv {
        println!("{:?}", re);
    }
}
