use log; // 0.4.14
use log::{debug, error, info, warn};
pub fn format_forbidden_char(filename: &str) -> String {
    // 禁止文字(半角記号)
    // let cannot_used_file_name = "\\/:*?`\"><|";
    // 禁止文字(全角記号)
    // let used_file_name = "￥／：＊？`”＞＜｜";
    //TODO motto smart ni yaritai
    filename
        .replace('\\', "￥")
        .replace('/', "／")
        .replace('\"', "”")
        .replace(':', "：")
        .replace('*', "＊")
        .replace('?', "？")
        .replace('`', "`")
        .replace('>', "＞")
        .replace('<', "＜")
}
#[test]
fn pass_format_char() {
    assert_eq!(format_forbidden_char("Fate/Test"), "Fate／Test")
}

pub fn get_fake_image(
    archive_path: &str,
    filename: &str,
    text: &str,
) -> Result<u64, reqwest::Error> {
    //https://dummyimage.com/600x400/000/fff&text=

    let imagefile = format!("{}/{}_thumb.jpg", &archive_path, &filename);
    let mut img = match std::fs::File::create(&imagefile) {
        Ok(mut f) => f,
        Err(e) => panic!("{}", e),
    };
    let url: String = format!("https://dummyimage.com/600x400/000/fff&text={}", &text);
    let mut res = match reqwest::blocking::get(url) {
        Ok(mut m) => m,
        Err(e) => {
            error!("{:#?}", e);
            panic!("{:#?}", e);
        }
    };

    res.copy_to(&mut img)
}

#[test]
fn test_fake_image() {
    let a = get_fake_image(".", "hoge", "this_is_test");
    assert_eq!(a.is_ok(), true);
}
