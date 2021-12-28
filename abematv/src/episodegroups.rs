use serde_derive::Deserialize;
use serde_derive::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub episode_group_contents: Vec<EpisodeGroupContent>,
}

impl Root {
    pub fn get_episode_ids(&self) -> Vec<String> {
        self.episode_group_contents
            .iter()
            .map(|x| x.id.clone())
            .collect()
    }
}
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeGroupContent {
    pub id: String,
    pub thumb_component: ThumbComponent,
    pub episode: Episode,
    #[serde(rename = "type")]
    pub type_field: i64,
    pub video: Video,
    pub info: Info,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbComponent {
    pub url_prefix: String,
    pub filename: String,
    pub extension: String,
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
pub struct Video {
    pub terms: Vec<Term>,
    pub broadcast_region_policy: Option<i64>,
    pub release_year: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Term {
    pub on_demand_type: i64,
    pub end_at: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    pub duration: i64,
}
