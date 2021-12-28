use chrono::{DateTime, Duration, Local};

use log::{debug, error, info};
use reqwest::Response;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
mod episode;
mod episodegroups;
mod media;
mod serie;
use rayon::prelude::*;

const STRTABLE: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

// const HKEY:String = b"3AF0298C219469522A313570E8583005A642E73EDD58E3EA2FB7339D3DF1597E";

const HKEY: &[u8; 64] = b"3AF0298C219469522A313570E8583005A642E73EDD58E3EA2FB7339D3DF1597E";
const _MEDIATOKEN_API: &str = "https://api.abema.io/v1/media/token";

const _LICENSE_API: &str = "https://license.abema.io/abematv-hls";
const _USER_API: &str = "https://api.abema.io/v1/users";
struct AbemaTVLicenseAdapter {}
impl AbemaTVLicenseAdapter {
    pub fn print_consts() {
        debug!("STRTABLE: {:#?}", STRTABLE);
    }
}

#[derive(Serialize, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbemaUserProfile {
    user_id: String,
    created_at: u32,
}
#[derive(Serialize, Debug, Deserialize)]
pub struct Profile {
    profile: AbemaUserProfile,
    token: String,
}
/// Get Stream
///
/// # Arguments
///
///
impl Profile {
    /// init
    pub async fn get_streams() -> Profile {
        // use reqwest;
        use std::collections::HashMap;

        let deviceid = uuid::Uuid::new_v4();
        let token = generate_applicationkeysecret(deviceid);
        let mut map = HashMap::new();
        map.insert("deviceId", deviceid.to_string());
        map.insert("applicationKeySecret", token);

        let client = reqwest::Client::new();
        let result = client
            .post(_USER_API)
            //  .body(json_data)
            .json(&map)
            .send();
        match result.await {
            Ok(x) => x.json::<Self>().await.unwrap(),
            Err(e) => {
                info!("{:#?}", e);
                panic!("{:#?}", e);
            }
        }
    }

    pub fn get_token(&self) -> String {
        self.token.clone()
    }
    pub async fn get_channel(&self) -> Response {
        let CHANNEL = "https://api.abema.io/v1/channels";
        let client = reqwest::Client::new();
        let result = client.get(CHANNEL).send();
        match result.await {
            Ok(x) => x,
            Err(e) => {
                info!("{:#?}", e);
                panic!("{:#?}", e);
            }
        }
    }
    pub async fn get_media(&self) -> Response {
        let CHANNEL = "https://api.abema.io/v1/media";
        let client = reqwest::Client::new();
        let result = client
            .get(CHANNEL)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Referer", "https://abema.tv/")
            .header("Origin", "https://abema.tv")
            .header("Accept-Encoding", "gzip")
            .send();
        match result.await {
            Ok(x) => x,
            Err(e) => {
                info!("{:#?}", e);
                panic!("{:#?}", e);
            }
        }
    }

    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
    pub async fn get_programs(&self, series_id: &str) -> Response {
        let dt: DateTime<Local> = Local::now();
        let _timestamp: i64 = dt.timestamp_nanos();
        let CHANNEL = format!("https://api.abema.io/v1/video/series/{}", series_id);
        // series/{}/programs?timestamp
        let client = reqwest::Client::new();
        let result = client
            .get(CHANNEL)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Referer", "https://abema.tv/")
            .header("Origin", "https://abema.tv")
            .header("Accept-Encoding", "gzip")
            .send();
        match result.await {
            Ok(x) => x,
            Err(e) => {
                info!("{:#?}", e);
                panic!("{:#?}", e);
            }
        }
    }
    pub async fn get_episodegroups(&self, episodegroups: &str) -> Response {
        let dt: DateTime<Local> = Local::now();
        let _timestamp: i64 = dt.timestamp_nanos();
        let CHANNEL = format!(
            "https://api.abema.io/v1/video/episodeGroups/{}/episodeGroupContents?limit=200",
            episodegroups
        );
        // series/{}/programs?timestamp
        let client = reqwest::Client::new();
        let result = client
            .get(CHANNEL)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Referer", "https://abema.tv/")
            .header("Origin", "https://abema.tv")
            .header("Accept-Encoding", "gzip")
            .send();
        match result.await {
            Ok(x) => x,
            Err(e) => {
                info!("{:#?}", e);
                panic!("{:#?}", e);
            }
        }
    }
    async fn episode_free_change(&self, episodeid: &str) -> bool {
        let res = self.get_episode(&episodeid).await;
//   let res2=&res.text().await.unwrap().clone();
        let a = match res.json::<episode::Root>().await {
            Ok(m) => m,
            Err(e) => {
                // println!("{:#?}",res2);
                error!("{}", e);
                panic!("{}", e);
            }
        };
        match a.free_end_at {
            Some(_m) => {
                info!("IS Free https://abema.tv/video/episode/{}", episodeid);
                true
            }
            None => {
                debug!("Not Free https://abema.tv/video/episode/{}", episodeid);
                false
            }
        }
    }
    /**
     * 
     */
    async fn get_episode(&self, episodeid: &str) -> Response {
        let _dt: DateTime<Local> = Local::now();
        // dbg!(episodeid);
        let channel = format!("https://api.abema.io/v1/video/programs/{}", episodeid);
        /*
        series/{}/programs?timestamp
        */
        let client = reqwest::Client::new();
        let result = client
            .get(channel)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Referer", "https://abema.tv/")
            .header("Origin", "https://abema.tv")
            .header("Accept-Encoding", "gzip")
            .send();
        match result.await {
            Ok(x) => x,
            Err(e) => {
                info!("{:#?}", e);
                panic!("{:#?}", e);
            }
        }
    }
}
#[derive(Serialize, Debug, Deserialize)]
struct AbemaChannels {
    channels: Vec<AbemaChannel>,
}
#[derive(Serialize, Debug, Deserialize)]
struct AbemaChannel {
    broadcastRegionPolicy: u8,
    gnid: Option<String>,
    id: String,
    name: String,
    playback: AbemaPlayback,
    status: AbemaChannelStatus,
}
#[derive(Serialize, Debug, Deserialize)]
struct AbemaChannelStatus {
    drm: Option<bool>,
}
#[derive(Serialize, Debug, Deserialize)]
struct AbemaPlayback {
    dash: String,
    #[serde(rename = "dashIPTV")]
    dash_iptv: String,
    hls: String,
    #[serde(rename = "hlsPreview")]
    hls_preview: String,
    //yospace: String,
}

async fn auth_get(url: &str, token: &str) -> Result<Response, reqwest::Error> {
    let client = reqwest::Client::new();
    let result = client
        .get(url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Referer", "https://abema.tv/")
        .header("Origin", "https://abema.tv")
        .send();

    result.await
}

#[tokio::main] 
async fn main() {
    env_logger::init();
    let expected_channel = "abema-anime";
    let profile = Profile::get_streams().await;
    // let test_response=profile.get_programs().await;
    // println!("{:#?}",test_response.text().await);
    let result = profile.get_channel().await;
    let result_media = profile.get_media().await;
    let media = match result_media.json::<media::Root>().await {
        Ok(m) => m,
        Err(e) => {
            error!("{}", e);
            panic!("{}", e);
        }
    };
    let dt: DateTime<Local> = Local::now();
    let _timestamp: i64 = dt.timestamp();
    let _dt2 = dt + Duration::days(1);
    let mut result_vec: Vec<String> = Vec::new();
    
    // future::stream::iter(media.channel_schedules)
     for i in media.channel_schedules {
    // .for_each(|i|  {
        for j in i.slots {
            if j.is_within_day() && j.channel_id.contains("anime") {
                println!("title {:#?}", j.title);
                println!(
                    "display:https://abema.tv/video/episode/{}",
                    j.display_program_id
                );

                for x in j.programs {
                    let k=x.series.id.clone();

                    let result_series = profile.get_programs(&k).await;
                    let series = match result_series.json::<serie::Root>().await {
                        Ok(m) => m,
                        Err(e) => {
                            error!("{}", e);
                            panic!("{}", e);
                        }
                    };

                    match series.seasons {
                        Some(m) => {
                            for f in m {
                                if let Some(j) = f.episode_groups {
                                    for k in j {
                                        let result_eg = profile.get_episodegroups(&k.id).await;

                                        let eg = match result_eg.json::<episodegroups::Root>().await
                                        {
                                            Ok(m) => m,
                                            Err(e) => {
                                                error!("{}", e);
                                                panic!("{}", e);
                                            } //TODO id をまseasonごとにまとめる
                                        };

                                        result_vec.extend(eg.get_episode_ids());
                                    }
                                } else {
                                    error!("missingepisodeid ");
                                }
                            }
                        }
                        None => println!("{}_s0", series.id),
                    }
                }

                result_vec.sort();
                result_vec.dedup();

                for x in &result_vec {
                    if profile.episode_free_change(x).await {
                        println!("{}", x);
                    }
                }

                result_vec.clear();

                // info!("https://abema.tv/channels/{}/slots/{}", j.channel_id, j.id);
                
            }
        }
    };

    let channels = result.json::<AbemaChannels>().await.unwrap();
    let channel = channels
        .channels
        .iter()
        .find(|x| x.id == *expected_channel)
        .unwrap();
    let playlisturl = &channel.playback.hls;
    // println!("playlisturl {:#?}", playlisturl);
    // info!("{:#?}",channels);
    let _result = auth_get(playlisturl, &profile.get_token()).await;

    // info!("result {:#?}", result.unwrap().text().await.unwrap());
}

const SECRETKEY: &[u8; 128] = b"v+Gjs=25Aw5erR!J8ZuvRrCx*rGswhB&qdHd_SYerEWdU&a?3DzN9BRbp5KwY4hEmcj5#fykMjJ=AuWz5GSMY-d@H7DMEh3M@9n2G552Us$$k9cD=3TxwWe86!x#Zyhe";
fn generate_applicationkeysecret(deviceid: Uuid) -> String {
    //use base64_url;
    use chrono::Datelike;
    use chrono::TimeZone;
    use chrono::Timelike;
    use chrono::Utc;
    use hmac::Hmac;
    use hmac::Mac;
    use sha2::Sha256;
    use std::thread::sleep;
    use std::time::Duration;
    use std::time::SystemTime;
    let now = std::time::SystemTime::now();
    debug!("{:#?}", now);
    let sec = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("SystemTime before UNIX EPOCH!")
        .as_secs();
    sleep(Duration::new(2, 0));
    debug!("{:#?}", sec);
    let ts_1hour: i64 = ((sec + 60 * 60) / 3600) as i64 * 3600;
    debug!("{:#?}", ts_1hour);

    let hoge = Utc.timestamp(ts_1hour, 0);
    debug!("hoge is {}", hoge.month());

    // Create alias for HMAC-SHA256
    type HmacSha256 = Hmac<Sha256>;
    let now = std::time::SystemTime::now();
    let sec = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("SystemTime before UNIX EPOCH!")
        .as_secs();
    let ts_1hour = ((sec + 60 * 60) / 3600) * 3600;
    // Create HMAC-SHA256 instance which implements `Mac` trait
    let mut mac = HmacSha256::new_from_slice(SECRETKEY).expect("HMAC can take key of any size");
    mac.update(SECRETKEY);

    let mut tmp_hoge = mac.finalize();

    let mut tmp = tmp_hoge.into_bytes();
    debug!("{:#?}", tmp);

    let month = hoge.month();
    for _i in 0..month {
        mac = HmacSha256::new_from_slice(SECRETKEY).expect("HMAC can take key of any size");
        mac.update(&tmp);
        tmp_hoge = mac.finalize();
        tmp = tmp_hoge.into_bytes();
    }

    mac = HmacSha256::new_from_slice(SECRETKEY).expect("HMAC can take key of any size");
    // debug!("{:#?}", tmp);

    let hoge2 = base64_url::encode(&tmp).trim_end_matches('=').to_string();
    let tmpascii = format!("{}{}", hoge2, deviceid);
    // info!("{}", tmpascii);
    mac.update(tmpascii.as_bytes());
    tmp_hoge = mac.finalize();
    tmp = tmp_hoge.into_bytes();
    // info!("{:#?}", tmp);

    let day = hoge.day() % 5;
    for _i in 0..day {
        mac = HmacSha256::new_from_slice(SECRETKEY).expect("HMAC can take key of any size");
        mac.update(&tmp);
        tmp_hoge = mac.finalize();
        tmp = tmp_hoge.into_bytes();
    }
    mac = HmacSha256::new_from_slice(SECRETKEY).expect("HMAC can take key of any size");
    let hoge2 = base64_url::encode(&tmp).trim_end_matches('=').to_string();
    let tmpascii = format!("{}{}", hoge2, ts_1hour);
    // debug!("{:#?}", tmpascii);
    mac.update(tmpascii.as_bytes());
    tmp_hoge = mac.finalize();
    tmp = tmp_hoge.into_bytes();
    // debug!("{:#?}", tmp);

    let hour = hoge.hour() % 5;
    for _i in 0..hour {
        mac = HmacSha256::new_from_slice(SECRETKEY).expect("HMAC can take key of any size");
        mac.update(&tmp);
        tmp_hoge = mac.finalize();
        tmp = tmp_hoge.into_bytes();
    }
    return base64_url::encode(&tmp).trim_end_matches('=').to_string();
}
