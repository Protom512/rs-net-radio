use log::error;
use reqwest::blocking::Response;
use serde::{Deserialize, Serialize};
extern crate m3u8_rs;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HibikiEpisode {
    // "id": 11858,
    pub id: i16,
    // "name": "第65回",
    name: String,
    // "media_type": null,
    media_type: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HibikiProgram {
    pub episode: HibikiProgramEpisode,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HibikiProgramEpisode {
    pub id: i16,
    pub video: HibikiProgramEpisodeVideo,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HibikiProgramEpisodeVideo {
    // "video": {
    // "id": 14894,
    // "duration": 1472.5,
    // "live_flg": false,
    // "delivery_start_at": null,
    // "delivery_end_at": null,
    // "dvr_flg": false,
    // "replay_flg": false,
    // "media_type": 1
    // },
    pub id: i16,
    pub live_flg: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HibikiVideoPlaylist {
    pub token: String,
    pub playlist_url: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]

pub struct Hibiki {
    pub name: String,
    pub episode: HibikiEpisode,
    pub access_id: String,
    pub latest_episode_id: i16,
    pub latest_episode_name: String,
    pub cast: String,
}
pub fn do_mkdir(file_name: &str) -> u8 {
    if !std::path::Path::new(&file_name).exists() {
        match std::fs::create_dir(file_name) {
            Err(e) => panic!("{}: {}", file_name, e),
            Ok(_) => 0,
        }
    } else {
        0
    }
}
pub fn get_api(url: &String) -> Response {
    let client = reqwest::blocking::Client::new();
    match client
        .get(url)
        .header(reqwest::header::CONTENT_TYPE, "application_json")
        .header(reqwest::header::ORIGIN, "https://hibiki-radio.jp")
        .header("X-Requested-with", "XMLHttpRequest")
        .send()
    {
        Ok(m) => m,
        Err(e) => {
            error!("{}", e);
            panic!("{}", e);
        }
    }
}
impl HibikiVideoPlaylist {
    pub fn get_m3u8(&self) -> String {
        format!(
            "{playlist_url}&token={token}",
            playlist_url = self.playlist_url,
            token = self.token
        )
    }
}
impl HibikiProgramEpisodeVideo {
    pub fn fetch_video_playlist(&self) -> HibikiVideoPlaylist {
        let res = get_api(&format!(
            "https://vcms-api.hibiki-radio.jp/api/v1/videos/play_check?video_id={video_id}",
            video_id = self.id
        ));
        match res.json::<HibikiVideoPlaylist>() {
            Ok(m) => m,
            Err(e) => {
                error!("{}", e);
                panic!("{}", e);
            }
        }
    }
}
impl Hibiki {
    ///
    /// HibikiのAPI結果取得関数
    /// ---
    /// 各ページ数の項目数を8と固定して、
    /// ページ番号を入力する
    ///
    pub fn fetch_api(page_number: i8) -> Response {
        let client = reqwest::blocking::Client::new();
        match client
            .get(format!(
                "https://vcms-api.hibiki-radio.jp/api/v1//programs?limit=8&page={}",
                page_number
            ))
            .header(reqwest::header::CONTENT_TYPE, "application_json")
            .header(reqwest::header::ORIGIN, "https://hibiki-radio.jp")
            .header("X-Requested-with", "XMLHttpRequest")
            .send()
        {
            Ok(m) => m,
            Err(e) => {
                error!("{}", e);
                panic!("{}", e);
            }
        }
    }
    pub fn get_list() -> Vec<Hibiki> {
        let mut list = Vec::new();
        let mut count = 1;
        loop {
            let res = Hibiki::fetch_api(count);

            match res.json::<Vec<Hibiki>>() {
                Ok(n) => list.append(&mut n.clone()),
                Err(e) => {
                    error!("{}", e);
                    panic!("{}", e);
                }
            }

            if list.len() >= 8 {
                break;
            } else {
                count += 1;
            }
        }
        list
    }
}
#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn fetch_api_test() {
        let result = Hibiki::fetch_api(2);

        assert_eq!(result.status(), 200);
    }
}
