use domain::HibikiProgram;
use env_logger;
use log::{debug, error, info};
use m3u8_rs::playlist::Playlist;
use std::env::set_var;
use std::fs::File;
use std::io::Read;
use std::io::{copy, Write};

use url::Url;

fn main() {
    set_var("RUST_LOG", "info");
    env_logger::init();

    let resp = domain::Hibiki::get_list();
    for i in resp {
        let work_dir = format!("/tmp/{}", i.access_id);
        let res = domain::get_api(&format!(
            "https://vcms-api.hibiki-radio.jp/api/v1/programs/{}",
            i.access_id
        ));
        let json = res.json::<HibikiProgram>().unwrap();

        info!(
            "{},{:#?}",
            i.name,
            assert_eq!(i.episode.id, json.episode.id)
        );
        if i.episode.id != json.episode.id {
            error!("episode outdated. title=#{title} expected_episode_id=#{episode_id} actual_episode_id=#{actual_episode_id}"
            ,title=i.name,episode_id=i.episode.id,actual_episode_id=json.episode.id);

            continue;
        }
        let live_flg = json.episode.video.live_flg;
        if live_flg {
            error!("{:#?} not downloadable", i.name);
            continue;
        }
        let url = json.episode.video.fetch_video_playlist().get_m3u8();
        // error!("{}", url);

        let mut playlist_m3u8 = domain::get_api(&url);
        let mut bytes: Vec<u8> = Vec::new();
        playlist_m3u8
            .read_to_end(&mut bytes)
            .expect("Unable to read data");
        let parsed = m3u8_rs::parse_playlist_res(&bytes);

        let ts = match parsed {
            Ok(Playlist::MasterPlaylist(pl)) => pl.variants,

            Ok(Playlist::MediaPlaylist(pl)) => panic!("Media playlist:\n{:?}", pl),
            Err(_e) => panic!("Error: "),
        };
        for variant in ts {
            let uri = Url::parse(&variant.uri).unwrap();

            let mut bytes: Vec<u8> = Vec::new();
            let mut ts_audio_m3u8 = domain::get_api(&variant.uri);
            ts_audio_m3u8
                .read_to_end(&mut bytes)
                .expect("Unable to read data");
            let ts_audio = m3u8_rs::parse_playlist_res(&bytes);

            match ts_audio {
                Ok(Playlist::MediaPlaylist(mut pl)) => {
                    domain::do_mkdir(&work_dir);
                    for mut seg in &mut pl.segments {
                        if seg.key.is_some() {
                            let key_url = &seg.key.as_ref().unwrap().uri.as_ref().unwrap();
                            let mut resp = domain::get_api(&key_url);
                            //temporary path

                            let mut tmp_vec: Vec<u8> = Vec::new();
                            resp.read_to_end(&mut tmp_vec)
                                .expect("unable to read key response");
                            info!("key: {:#?}", &tmp_vec);
                            let path = format!("{}/key", &work_dir);
                            // Write contents to disk.
                            let mut f = File::create(&path).expect("Unable to create file");
                            let _ = f.write(&resp.bytes().unwrap());
                            //copy(&mut resp, &mut f).expect("Unable to copy data");
                            seg.key.as_mut().map(|mut f| f.uri = Some(path));
                            // seg.key.map(|mut f| {
                            //     f.uri = Some(path);
                            // });
                            debug!("{:#?}", seg.key);
                        }

                        //                          None => {
                        let path_base = format!(
                            "{}://{}{}{}",
                            &uri.scheme(),
                            &uri.domain().unwrap(),
                            &uri.path().replace("ts_audio.m3u8", ""),
                            &seg.uri
                        );
                        debug!("{}", path_base);
                        let mut resp = domain::get_api(&path_base);
                        let path = format!("{}/{}", &work_dir, seg.uri);
                        debug!("output path will be {}", &path);
                        // Write contents to disk.
                        let mut f = File::create(&path).expect("Unable to create file");
                        copy(&mut resp, &mut f).expect("Unable to copy data");
                        seg.uri = path;
                        debug!("{:#?}", &seg);
                    }
                    //}
                    let mut tmp_file = tempfile::Builder::new()
                        .prefix("playlist.m3u")
                        .suffix(".tmp")
                        .tempfile_in(&work_dir)
                        .unwrap();

                    let _ = pl.write_to(&mut tmp_file);
                    let m3u8_path = format!("{}/ts_audio.m3u8", &work_dir);
                    let _res = std::fs::rename(&tmp_file.path(), &m3u8_path);
                }
                Ok(Playlist::MasterPlaylist(_pp)) => {
                    panic!("expected Media list, got Master list");
                }
                Err(_e) => error!("failed to parse"),
            }

            // download hls
        }
    }
}
