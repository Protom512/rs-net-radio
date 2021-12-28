use log::{debug, error, info, warn};
use reqwest::Response;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
const STRTABLE: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

// const HKEY:String = b"3AF0298C219469522A313570E8583005A642E73EDD58E3EA2FB7339D3DF1597E";

const HKEY: &'static [u8; 64] = b"3AF0298C219469522A313570E8583005A642E73EDD58E3EA2FB7339D3DF1597E";
const _MEDIATOKEN_API: &str = "https://api.abema.io/v1/media/token";

const _LICENSE_API: &str = "https://license.abema.io/abematv-hls";
const _USER_API: &str = "https://api.abema.io/v1/users";
struct AbemaTVLicenseAdapter {}

impl AbemaTVLicenseAdapter {
    //_MEDIATOKEN_SCHEMA = validate.Schema({"token": validate.text})

    //   _LICENSE_SCHEMA = validate.Schema({"k": validate.text,
    //                                    "cid": validate.text})
    pub fn print_consts() {
        dbg!("{:#?}", STRTABLE);
    }
}
#[derive(Serialize, Debug, Deserialize)]
pub struct AbemaUserProfile {
    userId: String,
    createdAt: u32,
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
        use reqwest;
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
                eprintln!("{:#?}", e);
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
                eprintln!("{:#?}", e);
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
    dashIPTV: String,
    hls: String,
    hlsPreview: String,
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
    let expected_channel = "abema-anime-2";
    let profile = Profile::get_streams().await;
    println!("{:#?}", profile);
    let result = profile.get_channel().await;
    let channels = result.json::<AbemaChannels>().await.unwrap();
    let channel = channels
        .channels
        .iter()
        .find(|x| x.id == expected_channel.to_string())
        .unwrap();
    let playlisturl = &channel.playback.hls;
    println!("playlisturl {:#?}", playlisturl);
    let result = auth_get(playlisturl, &profile.get_token()).await;

    println!("result {:#?}", result.unwrap().text().await.unwrap());
}
///```python
///    def _generate_applicationkeysecret(self, deviceid):
///        deviceid = deviceid.encode("utf-8")  # for python3
///        # plus 1 hour and drop minute and secs
///        # for python3 : floor division
///        ts_1hour = (int(time.time()) + 60 * 60) // 3600 * 3600
///        time_struct = time.gmtime(ts_1hour)
///        ts_1hour_str = str(ts_1hour).encode("utf-8")
///
///        h = hmac.new(self.SECRETKEY, digestmod=hashlib.sha256)
///        h.update(self.SECRETKEY)
///        tmp = h.digest()
///        for i in range(time_struct.tm_mon):
///            h = hmac.new(self.SECRETKEY, digestmod=hashlib.sha256)
///            h.update(tmp)
///            tmp = h.digest()
///        h = hmac.new(self.SECRETKEY, digestmod=hashlib.sha256)
///        h.update(urlsafe_b64encode(tmp).rstrip(b"=") + deviceid)
///        tmp = h.digest()
///        for i in range(time_struct.tm_mday % 5):
///            h = hmac.new(self.SECRETKEY, digestmod=hashlib.sha256)
///            h.update(tmp)
///            tmp = h.digest()
///
///        h = hmac.new(self.SECRETKEY, digestmod=hashlib.sha256)
///        h.update(urlsafe_b64encode(tmp).rstrip(b"=") + ts_1hour_str)
///        tmp = h.digest()
///
///        for i in range(time_struct.tm_hour % 5):  # utc hour
///            h = hmac.new(self.SECRETKEY, digestmod=hashlib.sha256)
///            h.update(tmp)
///            tmp = h.digest()
///
///        return urlsafe_b64encode(tmp).rstrip(b"=").decode("utf-8")
///```
///
///
///
///
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
    dbg!("{:#?}", now);
    let sec = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("SystemTime before UNIX EPOCH!")
        .as_secs();
    sleep(Duration::new(2, 0));
    dbg!("{:#?}", sec);
    let ts_1hour: i64 = ((sec + 60 * 60) / 3600) as i64 * 3600;
    dbg!("{:#?}", ts_1hour);

    let hoge = Utc.timestamp(ts_1hour, 0);
    dbg!("hoge is {}", hoge.month());

    // Create alias for HMAC-SHA256
    type HmacSha256 = Hmac<Sha256>;
    let now = std::time::SystemTime::now();
    let sec = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("SystemTime before UNIX EPOCH!")
        .as_secs();
    let ts_1hour = ((sec + 60 * 60) / 3600) as u64 * 3600;
    // Create HMAC-SHA256 instance which implements `Mac` trait
    let mut mac = HmacSha256::new_from_slice(SECRETKEY).expect("HMAC can take key of any size");
    mac.update(SECRETKEY);

    let mut tmp_hoge = mac.finalize();

    let mut tmp = tmp_hoge.into_bytes();
    dbg!("{}", tmp);

    let month = hoge.month();
    for _i in 0..month {
        mac = HmacSha256::new_from_slice(SECRETKEY).expect("HMAC can take key of any size");
        mac.update(&*tmp);
        tmp_hoge = mac.finalize();
        tmp = tmp_hoge.into_bytes();
    }

    mac = HmacSha256::new_from_slice(SECRETKEY).expect("HMAC can take key of any size");
    dbg!("{}", tmp);

    let hoge2 = base64_url::encode(&tmp).trim_end_matches('=').to_string();
    let tmpascii = format!("{}{}", hoge2, deviceid);
    println!("{}", tmpascii);
    mac.update(tmpascii.as_bytes());
    tmp_hoge = mac.finalize();
    tmp = tmp_hoge.into_bytes();
    println!("{:#?}", tmp);

    let day = hoge.day() % 5;
    for _i in 0..day {
        mac = HmacSha256::new_from_slice(SECRETKEY).expect("HMAC can take key of any size");
        mac.update(&*tmp);
        tmp_hoge = mac.finalize();
        tmp = tmp_hoge.into_bytes();
    }
    mac = HmacSha256::new_from_slice(SECRETKEY).expect("HMAC can take key of any size");
    let hoge2 = base64_url::encode(&tmp).trim_end_matches('=').to_string();
    let tmpascii = format!("{}{}", hoge2, ts_1hour);
    // dbg!("{}", tmpascii);
    mac.update(tmpascii.as_bytes());
    tmp_hoge = mac.finalize();
    tmp = tmp_hoge.into_bytes();
    dbg!("{:#?}", tmp);

    let hour = hoge.hour() % 5;
    for _i in 0..hour {
        mac = HmacSha256::new_from_slice(SECRETKEY).expect("HMAC can take key of any size");
        mac.update(&*tmp);
        tmp_hoge = mac.finalize();
        tmp = tmp_hoge.into_bytes();
    }
    return base64_url::encode(&tmp).trim_end_matches('=').to_string();
}
