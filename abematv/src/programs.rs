use serde_derive::Deserialize;
use serde_derive::Serialize;



///v1/video/series/54-79/programs
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Programs {
    pub programs: Vec<Program>,
    pub version: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Program {
    pub id: String,
    pub series: Series,
    pub season: Season,
    pub info: Info,
    pub provided_info: ProvidedInfo,
    pub episode: Episode,
    pub credit: Credit,
    pub media_status: MediaStatus,
    pub label: Label2,
    pub image_updated_at: i64,
    pub end_at: i64,
    pub free_end_at: i64,
    pub transcode_version: String,
    pub shared_link: SharedLink,
    pub viewing_point: ViewingPoint,
    pub download: Download,
    pub external_content: ExternalContent,
    pub broadcast_region_policy: i64,
    pub timeline_thumb_component: TimelineThumbComponent,
    pub terms: Vec<Term>,
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
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbComponent {
    pub url_prefix: String,
    pub filename: String,
    pub query: String,
    pub extension: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbPortraitComponent {
    pub url_prefix: String,
    pub filename: String,
    pub query: String,
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
pub struct ThumbComponent2 {
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
    pub scene_thumb_imgs: Vec<String>,
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
pub struct MediaStatus {
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Label2 {
    pub free: bool,
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
pub struct ViewingPoint {
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Download {
    pub enable: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalContent {
    pub marks: Marks,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Marks {
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineThumbComponent {
    pub url_prefix: String,
    pub query: String,
    pub extension: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Term {
    pub on_demand_type: i64,
    pub end_at: i64,
}
