use serde_derive::Deserialize;
use serde_derive::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub id: String,
    pub genre: Genre,
    pub title: String,
    pub content: String,
    pub seasons: Option<Vec<Season>>,
    pub ordered_seasons: Option<Vec<OrderedSeason>>,
    pub copyrights: Vec<String>,
    pub label: Label,
    pub version: String,
    pub image_updated_at: i64,
    pub program_order: String,
    pub shared_link: SharedLink,
    pub thumb_component: ThumbComponent3,
    pub thumb_portrait_component: ThumbPortraitComponent,
    pub on_demand_types: Vec<i64>,
    // pub content_pattern: i64,
}

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
pub struct Season {
    pub id: String,
    pub sequence: i64,
    pub name: String,
    pub thumb_component: ThumbComponent,
    pub episode_groups: Option<Vec<EpisodeGroup>>,
    pub order: i64,
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
pub struct EpisodeGroup {
    pub id: String,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderedSeason {
    pub id: String,
    pub sequence: i64,
    pub name: String,
    pub thumb_component: ThumbComponent2,
    pub episode_groups: Option<Vec<EpisodeGroup>>,
    pub order: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbComponent2 {
    pub url_prefix: String,
    pub filename: String,
    pub query: String,
    pub extension: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeGroup2 {
    pub id: String,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    pub some_free: Option<bool>,
    pub newest: Option<bool>,
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
pub struct ThumbComponent3 {
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
