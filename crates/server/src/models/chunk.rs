use ragit::{Chunk, ChunkSource, Uid};
use serde::Serialize;
use std::collections::HashSet;

// `ragit::Chunk` is becoming more and more complicated and I don't want to
// expose too much internals of ragit to users. So I have decided to create
// another schema for chunk apis. I know fragmentation is bad, but I don't
// want to teach users how to get a chunk uid from 2 u128 integers.
#[derive(Clone, Debug, Serialize)]
pub struct ChunkDetail {
    pub uid: String,
    pub data: Vec<ChunkData>,
    pub image_uids: Vec<String>,
    pub title: String,
    pub summary: String,
    pub file: Option<String>,
    pub file_index: Option<usize>,
    pub timestamp: i64,
    pub model: String,
    pub ragit_version: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum ChunkData {
    Text { content: String },
    Image { uid: String },
}

impl From<Chunk> for ChunkDetail {
    fn from(c: Chunk) -> ChunkDetail {
        let (file, file_index) = match &c.source {
            ChunkSource::File { path, index } => (Some(path.to_string()), Some(*index)),
            _ => (None, None),
        };

        ChunkDetail {
            uid: c.uid.to_string(),
            data: into_chunk_data(&c.data, &c.images),
            image_uids: c.images.iter().map(|uid| uid.to_string()).collect(),
            title: c.title.clone(),
            summary: c.summary.clone(),
            file,
            file_index,
            timestamp: c.timestamp,
            model: c.build_info.model.clone(),
            ragit_version: c.build_info.ragit_version.clone(),
        }
    }
}

// It replaces `"img_{uid}"` with `ChunkData::Image { uid }`
fn into_chunk_data(data: &str, images: &[Uid]) -> Vec<ChunkData> {
    let bytes = data.as_bytes();
    let images = images.iter().map(|uid| uid.to_string().into_bytes()).collect::<HashSet<_>>();
    let mut index = 0;
    let mut last_index = 0;
    let mut result = vec![];

    while index < bytes.len() {
        match bytes[index] {
            b'i' => match (bytes.get(index + 1), bytes.get(index + 2), bytes.get(index + 3)) {
                (Some(b'm'), Some(b'g'), Some(b'_')) => match bytes.get(index + 67) {
                    Some(b'0'..=b'f') => {
                        if images.contains(&bytes[(index + 4)..(index + 68)]) {
                            if last_index < index {
                                result.push(ChunkData::Text { content: String::from_utf8(bytes[last_index..index].to_vec()).unwrap() });
                            }

                            result.push(ChunkData::Image { uid: String::from_utf8(bytes[(index + 4)..(index + 68)].to_vec()).unwrap() });
                            last_index = index + 68;
                            index += 68;
                        }

                        else {
                            index += 4;
                        }
                    },
                    _ => {
                        index += 4;
                    },
                },
                _ => { index += 1; },
            },
            _ => { index += 1; },
        }
    }

    if last_index < bytes.len() {
        result.push(ChunkData::Text { content: String::from_utf8(bytes[last_index..].to_vec()).unwrap() });
    }

    result
}

#[test]
fn into_chunk_data_test() {
    let uid1 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa0000000200000000";
    let uid2 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb0000000200000000";
    let fake_uid = "cccccccccccccccccccccccccccccccccccccccccccccccc0000000200000000";
    let images = vec![
        uid1.parse::<Uid>().unwrap(),
        uid2.parse::<Uid>().unwrap(),
    ];

    assert_eq!(
        into_chunk_data("", &images),
        vec![],
    );
    assert_eq!(
        into_chunk_data("Hi, my name is baehyunsol", &images),
        vec![ChunkData::Text { content: String::from("Hi, my name is baehyunsol") }],
    );
    assert_eq!(
        into_chunk_data(&format!("img_{uid1}"), &images),
        vec![ChunkData::Image { uid: String::from(uid1) }],
    );
    assert_eq!(
        into_chunk_data(&format!("img_{uid1}img_{uid1}"), &images),
        vec![ChunkData::Image { uid: String::from(uid1) }, ChunkData::Image { uid: String::from(uid1) }],
    );
    assert_eq!(
        into_chunk_data(&format!("aaimg_{uid1}img_{uid1}aa"), &images),
        vec![ChunkData::Text { content: String::from("aa") }, ChunkData::Image { uid: String::from(uid1) }, ChunkData::Image { uid: String::from(uid1) }, ChunkData::Text { content: String::from("aa") }],
    );
    assert_eq!(
        into_chunk_data(&format!("img_{uid1}imgimg_{uid1}"), &images),
        vec![ChunkData::Image { uid: String::from(uid1) }, ChunkData::Text { content: String::from("img") }, ChunkData::Image { uid: String::from(uid1) }],
    );
    assert_eq!(
        into_chunk_data(&format!("Hi, my name is baehyunsolimg_{uid1}"), &images),
        vec![ChunkData::Text { content: String::from("Hi, my name is baehyunsol") }, ChunkData::Image { uid: String::from(uid1) }],
    );
    assert_eq!(
        into_chunk_data(&format!("img_{uid1}Hi, my name is baehyunsol"), &images),
        vec![ChunkData::Image { uid: String::from(uid1) }, ChunkData::Text { content: String::from("Hi, my name is baehyunsol") }],
    );
    assert_eq!(
        into_chunk_data(&format!("img_{uid1}Hi, my name is baehyunsolimg_{uid1}"), &images),
        vec![ChunkData::Image { uid: String::from(uid1) }, ChunkData::Text { content: String::from("Hi, my name is baehyunsol") }, ChunkData::Image { uid: String::from("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa0000000200000000") }],
    );
    assert_eq!(
        into_chunk_data(&format!("img_{fake_uid}"), &images),
        vec![ChunkData::Text { content: format!("img_{fake_uid}") }],
    );
    assert_eq!(
        into_chunk_data(&format!("img_{uid1}img_{fake_uid}img_{uid2}"), &images),
        vec![ChunkData::Image { uid: String::from(uid1) }, ChunkData::Text { content: format!("img_{fake_uid}") }, ChunkData::Image { uid: String::from(uid2) }],
    );
}
