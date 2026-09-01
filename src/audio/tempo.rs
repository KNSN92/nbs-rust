use crate::{Nbs, Tick};

#[derive(Debug, Clone)]
pub struct TempoMap(Vec<(Tick, f32)>);

impl TempoMap {
    //TODO: テストを追加する。テスト用のnbsファイル、もしくはデータの作成が大変なので一旦保留。
    /// Creates a new TempoMap from the given Nbs struct.
    pub fn from_nbs(nbs: &Nbs) -> Self {
        let default_tempo = nbs.header.song_meta.tempo;
        let mut tempo_map = Vec::new();
        for (&tick, notes_in_tick) in nbs.note_blocks.inner_tick_notes() {
            let tempo = notes_in_tick.iter().find_map(|(_, note)| {
                nbs.instrument_set
                    .is_tempo_changer(note.instrument)
                    .then_some(note.pitch as f32 / 15.0)
            });
            if let Some(tempo) = tempo {
                tempo_map.push((tick, tempo));
            }
        }
        tempo_map.sort_by_key(|(tick, _)| *tick);
        match tempo_map.first() {
            Some((tick, _)) if *tick == 0 => {}
            _ => tempo_map.insert(0, (0, default_tempo)),
        }
        TempoMap(tempo_map)
    }

    /// Returns the tempo at the given tick.
    pub fn get_tempo_at(&self, tick: Tick) -> f32 {
        let index = self
            .0
            .binary_search_by(|(t, _)| t.cmp(&tick))
            .unwrap_or_else(|e| e.saturating_sub(1));
        self.0[index].1
    }

    /// Returns true if the given tick is a tempo changing tick, false otherwise.
    pub fn is_tempo_changing_tick(&self, tick: Tick) -> bool {
        self.0.binary_search_by(|(t, _)| t.cmp(&tick)).is_ok()
    }

    //TODO: 0tick目からあるtick目までの再生時間をDurationで取得する関数を追加したい。
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tempo_map() {
        let tempo_map = TempoMap(vec![(0, 10.0), (2, 20.0), (4, 5.0), (6, 40.0)]);
        assert_eq!(tempo_map.get_tempo_at(0), 10.0);
        assert_eq!(tempo_map.get_tempo_at(1), 10.0);
        assert_eq!(tempo_map.get_tempo_at(2), 20.0);
        assert_eq!(tempo_map.get_tempo_at(3), 20.0);
        assert_eq!(tempo_map.get_tempo_at(4), 5.0);
        assert_eq!(tempo_map.get_tempo_at(5), 5.0);
        assert_eq!(tempo_map.get_tempo_at(6), 40.0);
        assert_eq!(tempo_map.get_tempo_at(7), 40.0);
        assert_eq!(tempo_map.get_tempo_at(Tick::MAX), 40.0);
    }

    #[test]
    fn test_tempo_map_with_no_tempo_changes() {
        let tempo_map = TempoMap(vec![(0, 10.0)]);
        assert_eq!(tempo_map.get_tempo_at(0), 10.0);
        assert_eq!(tempo_map.get_tempo_at(1), 10.0);
        assert_eq!(tempo_map.get_tempo_at(100), 10.0);
        assert_eq!(tempo_map.get_tempo_at(Tick::MAX), 10.0);
    }

    #[test]
    fn test_is_tempo_changing_tick() {
        let tempo_map = TempoMap(vec![(0, 10.0), (2, 20.0), (4, 5.0), (6, 40.0)]);
        assert!(tempo_map.is_tempo_changing_tick(0));
        assert!(!tempo_map.is_tempo_changing_tick(1));
        assert!(tempo_map.is_tempo_changing_tick(2));
        assert!(!tempo_map.is_tempo_changing_tick(3));
        assert!(tempo_map.is_tempo_changing_tick(4));
        assert!(!tempo_map.is_tempo_changing_tick(5));
        assert!(tempo_map.is_tempo_changing_tick(6));
        assert!(!tempo_map.is_tempo_changing_tick(7));
    }
}
