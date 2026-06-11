use std::num::NonZeroU8;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SongMetadata {
    pub vanilla_instruments: u8,
    pub length: u16,
    pub layers: u16,
    pub tempo: f32,
    pub looping: Looping,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SongInfo {
    pub name: String,
    pub author: String,
    pub original_author: String,
    pub description: String,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SongStats {
    pub minutes_spent: u32,
    pub left_clicks: u32,
    pub right_clicks: u32,
    pub note_blocks_added: u32,
    pub note_blocks_removed: u32,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EditorInfo {
    pub time_signature: u8,
    pub auto_saving: AutoSaving,
    pub midi_schematic_file_name: String,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Looping {
    pub enabled: bool,
    pub count: Option<NonZeroU8>,
    pub start_tick: u16,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AutoSaving {
    pub enabled: bool,
    pub duration: u8,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Header {
    pub song_meta: SongMetadata,
    pub song_info: SongInfo,
    pub song_stats: SongStats,
    pub editor_info: EditorInfo,
}

impl Default for Header {
    fn default() -> Self {
        Header {
            song_meta: SongMetadata {
                vanilla_instruments: 16,
                length: 0,
                layers: 0,
                tempo: 10.0,
                looping: Looping {
                    enabled: false,
                    count: None,
                    start_tick: 0,
                },
            },
            song_info: SongInfo {
                name: "".to_string(),
                author: "".to_string(),
                original_author: "".to_string(),
                description: "".to_string(),
            },
            song_stats: SongStats {
                minutes_spent: 0,
                left_clicks: 0,
                right_clicks: 0,
                note_blocks_added: 0,
                note_blocks_removed: 0,
            },
            editor_info: EditorInfo {
                time_signature: 4,
                auto_saving: AutoSaving {
                    enabled: false,
                    duration: 10,
                },
                midi_schematic_file_name: "".to_string(),
            },
        }
    }
}
