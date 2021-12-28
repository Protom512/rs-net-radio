use chrono::DateTime;
use chrono::Duration;
use chrono::Local;
use serde_derive::Deserialize;
use serde_derive::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub channels: Vec<Channel>,
    pub channel_schedules: Vec<ChannelSchedule>,
    pub available_dates: Vec<String>,
    pub version: String,
    pub published_at: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub media_status: MediaStatus,
    pub broadcast_region_policy: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaStatus {
    #[serde(rename = "dashISOFF")]
    pub dash_isoff: Option<bool>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelSchedule {
    pub channel_id: String,
    pub date: String,
    pub slots: Vec<Slot>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Slot {
    pub id: String,
    pub title: String,
    pub start_at: i64,
    pub end_at: i64,
    pub programs: Vec<Program>,
    pub table_start_at: i64,
    pub table_end_at: i64,
    pub highlight: String,
    pub detail_highlight: Option<String>,
    pub content: String,
    pub display_program_id: String,
    pub mark: Mark,
    pub flags: Flags,
    pub channel_id: String,
    pub slot_group: Option<SlotGroup>,
    #[serde(default)]
    pub links: Option<Vec<Link>>,
    pub shared_link: SharedLink,
    pub external_content: ExternalContent,
    pub reservable: bool,
    pub broadcast_region_policies: BroadcastRegionPolicies,
    pub timeline_thumb_component: TimelineThumbComponent,
    pub hashtag: Option<String>,
    pub timeshift_end_at: Option<i64>,
    pub timeshift_free_end_at: Option<i64>,
    pub table_highlight: Option<String>,
}
impl Slot {
    pub fn get_url(&self) -> String {
        return format!(
            "https://abema.tv/channel/{}/slots/{}",
            self.channel_id, self.id
        );
    }
    pub fn get_title(&self) -> String {
        self.title.clone()
    }
    pub fn is_within_day(&self) -> bool {
        let dt: DateTime<Local> = Local::now();
        let timestamp: i64 = dt.timestamp();
        let dt2 = dt + Duration::days(1);
        let timestamp_aday: i64 = dt2.timestamp();
        return match self.timeshift_free_end_at {
            Some(m) => {
                //TODO: if with in a day
                if timestamp_aday > m && m > timestamp {
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Program {
    pub id: String,
    pub episode: Episode,
    pub credit: Credit,
    pub series: Series,
    pub provided_info: ProvidedInfo,
}
// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_slot_equality() {
//         let slot1 = Slot {
//             id: "1".to_string(),
//             title: "Test Slot".to_string(),
//             start_at: 1631654400,
//             end_at: 1631658000,
//             programs: vec![Program { id: "1".to_string(), name: "Test Program".to_string() }],
//             table_start_at: 1631653200,
//             table_end_at: 1631661600,
//             highlight: "red".to_string(),
//             detail_highlight: Some("blue".to_string()),
//             content: "Test content".to_string(),
//             display_program_id: "1".to_string(),
//             mark: Mark::New,
//             flags: Flags { featured: true },
//             channel_id: "1".to_string(),
//             slot_group: None,
//             links: Some(vec![Link { rel: "self".to_string(), href: "http://example.com".to_string() }]),
//             shared_link: SharedLink { href: "http://example.com".to_string(), text: "Test link".to_string() },
//             external_content: ExternalContent { url: "http://example.com/test.mp4".to_string(), duration_ms: 60000 },
//             reservable: true,
//             broadcast_region_policies: BroadcastRegionPolicies { policies: vec![] },
//             timeline_thumb_component: TimelineThumbComponent {
//                 url: "http://example.com/thumb.png".to_string(),
//                 width: 100,
//                 height: 50,
//                 duration_ms: 5000,
//                 x_offset: 0.5,
//                 y_offset: 0.25,
//             },
//             hashtag: Some("test".to_string()),
//             timeshift_end_at: Some(1631658000),
//             timeshift_free_end_at: None,
//             table_highlight: None,
//         };
//         let slot2 = Slot {
//             id: "1".to_string(),
//             title: "Test Slot".to_string(),
//             start_at: 1631654400,
//             end_at: 1631658000,
//             programs: vec![Program { id: "1".to_string(), name: "Test Program".to_string() }],
//             table_start_at: 1631653200,
//             table_end_at: 1631661600,
//             highlight: "red".to_string(),
//             detail_highlight: Some("blue".to_string()),
//             content: "Test content".to_string(),
//             display_program_id: "1".to_string(),
//             mark: Mark::New,
//             flags: Flags { featured: true },
//             channel_id: "1".to_string(),
//             slot_group: None,
//             links: Some(vec![Link { rel: "self".to_string(), href: "http://example.com".to_string() }]),
//             shared_link: SharedLink { href: "http://example.com".to_string(), text: "Test link".to_string() },
//             external_content: ExternalContent { url: "http://example.com/test.mp4".to_string(), duration_ms: 60000 },
//             reservable: true,
//             broadcast_region_policies: BroadcastRegionPolicies { policies: vec![] },
//             timeline_thumb_component: TimelineThumbComponent {
//                 url: "http://example.com/thumb.png".to_string(),
//                 width: 100,
//                 height: 50,
//                 duration_ms: 5000,
//                 x_offset: 0.5,
//                 y_offset: 0.25,
//             },
//             hashtag: Some("test".to_string()),
//             timeshift_end_at: Some(1631658000),
//             timeshift_free_end_at: None,
//             table_highlight: None,
//         };
//         assert_eq!(slot1, slot2);
//     }

// #[test]
// fn test_slot_default() {
//     let slot = Slot::default();
//     assert_eq!(slot.id, "");
//     assert_eq!(slot.title, "");
//     assert_eq!(slot.start_at, 0);
//     assert_eq!(slot.end_at, 0);
//     assert_eq!(slot.programs, vec![]);
//     assert_eq!(slot.table_start_at, 0);
//     assert_eq!(slot.table_end_at, 0);
//     assert_eq!(slot.highlight, "");
//     assert_eq!(slot.detail_highlight, None);
//     assert_eq!(slot.content, "");
//     assert_eq!(slot.display_program_id, "");
//     assert_eq!(slot.mark, Mark::None);
//     assert_eq!(slot.flags, Flags { featured: false });
//     assert_eq!(slot.channel_id, "");
//     assert_eq!(slot.slot_group, None);
//     assert_eq!(slot.links, None);
//     assert_eq!(slot.shared_link, SharedLink { href: "".to_string(), text: "".to_string() });
//     assert_eq!(slot.external_content, ExternalContent { url: "".to_string(), duration_ms: 0 });
//     assert_eq!(slot.reservable, false);
//     assert_eq!(slot.broadcast_region_policies.policies, vec![]);
//     assert_eq!(
//         slot.timeline_thumb_component,
//         TimelineThumbComponent {url:"".to_string(),width:0,height:0,duration_ms:0,x_offset:0.0,y_offset:0.0, url_prefix: "".to_string(), extension: "".to_string() }
//     );
//     assert_eq!(slot.hashtag, None);
//     assert_eq!(slot.timeshift_end_at, None);
//     assert_eq!(slot.timeshift_free_end_at, None);
//     assert_eq!(slot.table_highlight, None);
// }
// }

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    pub sequence: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credit {
    pub casts: Vec<String>,
    #[serde(default)]
    pub crews: Vec<String>,
    pub copyrights: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Series {
    pub id: String,
    pub theme_color: ThemeColor,
    pub genre_id: String,
    pub updated_at: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeColor {}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvidedInfo {
    pub thumb_img: String,
    pub updated_at: i64,
    #[serde(default)]
    pub scene_thumb_imgs: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mark {
    pub live: Option<bool>,
    pub recommendation: Option<bool>,
    pub newcomer: Option<bool>,
    pub binge_watching: Option<bool>,
    pub drm: Option<bool>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Flags {
    pub archive_comment: Option<bool>,
    pub disable_trim: Option<bool>,
    pub timeshift: Option<bool>,
    pub timeshift_free: Option<bool>,
    pub chase_play: Option<bool>,
    pub share: Option<bool>,
    pub sharing_policy: Option<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotGroup {
    pub id: String,
    pub last_slot_id: String,
    pub title: String,
    pub thumb_component: ThumbComponent,
    pub expire_at: i64,
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
pub struct Link {
    pub value: String,
    #[serde(rename = "type")]
    pub type_field: i64,
    pub title: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedLink {
    pub twitter: String,
    pub facebook: String,
    pub google: String,
    pub line: String,
    pub copy: String,
    pub screen: String,
    pub instagram: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalContent {
    pub button_text: String,
    pub marks: Marks,
    pub link_text: Option<String>,
    pub link: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Marks {
    pub gambling: Option<bool>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastRegionPolicies {
    pub linear: i64,
    pub timeshift: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineThumbComponent {
    pub url_prefix: String,
    pub extension: String,
}
