use serde_derive::Deserialize;
use serde_derive::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub id: String,
    pub series: Series,
    pub season: Season,
    pub genre: Genre,
    pub info: Info,
    pub provided_info: ProvidedInfo,
    pub episode: Episode,
    pub credit: Credit,
    pub media_status: MediaStatus,
    pub label: Label2,
    pub image_updated_at: i64,
    pub end_at: i64,
    pub free_end_at: Option<i64>,
    pub transcode_version: String,
    pub shared_link: SharedLink,
    pub playback: Playback,
    pub viewing_point: ViewingPoint,
    pub next_program_info: Option<NextProgramInfo>,
    pub download: Download,
    pub external_content: ExternalContent,
    pub broadcast_region_policy: i64,
    pub timeline_thumb_component: TimelineThumbComponent,
    pub terms: Vec<Term2>,
    pub episode_group_id: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Series {
    pub id: String,
    pub title: String,
    pub label: Label,
    pub thumb_component: ThumbComponent,
    pub thumb_portrait_component: ThumbPortraitComponent,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    pub some_free: Option<bool>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbComponent {
    pub url_prefix: String,
    pub filename: String,
    pub query: Option<String>,
    pub extension: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbPortraitComponent {
    pub url_prefix: String,
    pub filename: String,
    pub query: Option<String>,
    pub extension: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Season {
    pub id: String,
    pub sequence: i64,
    pub name: String,
    pub thumb_component: ThumbComponent2,
    pub order: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbComponent2 {}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Genre {
    pub id: String,
    pub name: String,
    pub sub_genres: Vec<SubGenre>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubGenre {
    pub id: String,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    pub duration: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvidedInfo {
    pub thumb_img: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    pub number: i64,
    pub title: String,
    pub content: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credit {
    pub released: i64,
    pub casts: Vec<String>,
    pub crews: Vec<String>,
    pub copyrights: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaStatus {}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Label2 {
    pub free: Option<bool>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedLink {
    pub twitter: String,
    pub facebook: String,
    pub google: String,
    pub line: String,
    pub copy: String,
    pub instagram: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playback {
    pub hls: String,
    pub dash: String,
    pub hls_preview: Option<String>,
    #[serde(rename = "dashIPTV")]
    pub dash_iptv: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewingPoint {
    pub suggestion: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextProgramInfo {
    pub program_id: String,
    pub title: String,
    pub thumb_img: String,
    pub image_updated_at: i64,
    pub end_at: i64,
    pub broadcast_region_policy: i64,
    pub terms: Vec<Term>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Term {
    pub on_demand_type: i64,
    pub end_at: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Download {
    pub enable: Option<bool>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalContent {
    pub marks: Marks,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Marks {}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineThumbComponent {
    pub url_prefix: String,
    pub query: Option<String>,
    pub extension: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Term2 {
    pub on_demand_type: i64,
    pub end_at: i64,
}
