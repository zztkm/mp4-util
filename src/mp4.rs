use shiguredo_mp4::{
    Decode, Mp4File,
    aux::SampleTableAccessor,
    boxes::{RootBox, SampleEntry, TrakBox},
};
use std::io::Read;

pub struct InputMp4 {
    tracks: Vec<TrakBox>,
}

impl InputMp4 {
    pub fn parse<R: Read>(reader: R) -> Result<Self, String> {
        let mp4_file = match Mp4File::decode(reader) {
            Ok(file) => file,
            Err(e) => return Err(format!("MP4 ファイルの解析に失敗しました: {}", e)),
        };
        let moov_box = mp4_file.boxes.iter().find_map(|box_item| {
            if let RootBox::Moov(moov_box) = box_item {
                Some(moov_box)
            } else {
                None
            }
        });
        if moov_box.is_none() {
            return Err("moov box not found".to_string());
        }
        let moov_box = moov_box.unwrap();

        let mut tracks = Vec::new();
        for trak in moov_box.trak_boxes.iter() {
            // トラック情報を取得
            tracks.push(trak.clone());
        }

        Ok(InputMp4 { tracks })
    }

    /// MP4 ファイルのトラック情報を取得する
    pub fn get_track_infos(&self) -> Option<Vec<TrackInfo>> {
        let mut tracks = Vec::new();
        for trak in self.tracks.iter() {
            // トラック情報を取得
            tracks.push(self.get_track_info(trak));
        }
        Some(tracks)
    }

    fn get_track_info(&self, trak: &TrakBox) -> TrackInfo {
        // メディアタイプ (ビデオ/オーディオ)
        let handler_type = &trak.mdia_box.hdlr_box.handler_type;
        let media_type = match handler_type {
            b"vide" => "ビデオ",
            b"soun" => "オーディオ",
            _ => "不明",
        }
        .to_string();

        // トラックの時間情報を取得
        let track_timescale = trak.mdia_box.mdhd_box.timescale.get() as f64;
        let track_duration = trak.mdia_box.mdhd_box.duration as f64 / track_timescale;

        // サンプルエントリからコーデック情報を取得
        let codec = match trak.mdia_box.minf_box.stbl_box.stsd_box.entries.first() {
            Some(sample_entry) => self.get_codec_name(sample_entry),
            None => "不明 (サンプルエントリなし)".to_string(),
        };

        // サンプルテーブルから詳細情報を取得
        let (sample_count, chunk_count) =
            match SampleTableAccessor::new(&trak.mdia_box.minf_box.stbl_box) {
                Ok(sample_table) => (
                    Some(sample_table.sample_count()),
                    Some(sample_table.chunk_count()),
                ),
                Err(_) => (None, None),
            };

        TrackInfo {
            media_type,
            duration: track_duration,
            codec,
            sample_count,
            chunk_count,
        }
    }

    fn get_codec_name(&self, sample_entry: &SampleEntry) -> String {
        match sample_entry {
            SampleEntry::Avc1(_) => "AVC(H.264)".to_string(),
            SampleEntry::Hev1(_) => "HEVC(H.265)".to_string(),
            SampleEntry::Vp08(_) => "VP8".to_string(),
            SampleEntry::Vp09(_) => "VP9".to_string(),
            SampleEntry::Av01(_) => "AV1".to_string(),
            SampleEntry::Opus(_) => "Opus".to_string(),
            SampleEntry::Mp4a(_) => "MPEG AAC Audio (mp4a)".to_string(),
            SampleEntry::Unknown(unknown) => {
                let box_type = String::from_utf8_lossy(unknown.box_type.as_bytes());
                format!("不明 ({})", box_type)
            }
        }
    }
}

/// トラック情報を格納する構造体
pub struct TrackInfo {
    pub media_type: String,
    pub duration: f64,
    pub codec: String,
    pub sample_count: Option<u32>,
    pub chunk_count: Option<u32>,
}
