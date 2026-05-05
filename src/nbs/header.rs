#[derive(Debug, Default)]
pub struct SongMetadata {
    pub vanilla_instruments: u8,
    pub length: u16,
    pub layers: u16,
    pub tempo: f32,
    pub time_signature: u8,
}

#[derive(Debug)]
pub struct SongInfo {
    pub name: String,
    pub author: String,
    pub original_author: String,
    pub description: String,
}

#[derive(Debug)]
pub struct SongStats {
    pub minutes_spent: u32,
    pub left_clicks: u32,
    pub right_clicks: u32,
    pub note_blocks_added: u32,
    pub note_blocks_removed: u32,
}

#[derive(Debug)]
pub enum Looping {
    NoLooping,
    Finite { count: u8, loop_start_tick: u16 },
    Infinite { loop_start_tick: u16 },
}

#[derive(Debug)]
pub enum AutoSaving {
    Disabled { duration: u8 },
    Enabled { duration: u8 },
}

#[derive(Debug)]
pub struct Header {
    pub song_meta: SongMetadata,
    pub song_info: SongInfo,
    pub song_stats: SongStats,
    pub auto_saving: AutoSaving,
    pub looping: Looping,
    pub midi_schematic_file_name: String,
}

impl Default for Header {
    fn default() -> Self {
        Header {
            song_meta: SongMetadata {
                vanilla_instruments: 16,
                length: 0,
                layers: 0,
                tempo: 10.0,
                time_signature: 4,
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
            auto_saving: AutoSaving::Disabled { duration: 10 },
            looping: Looping::NoLooping,
            midi_schematic_file_name: "".to_string(),
        }
    }
}
