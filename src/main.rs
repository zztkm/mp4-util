use noargs;
use shiguredo_mp4::{
    Decode, Mp4File,
    aux::SampleTableAccessor,
    boxes::{MoovBox, RootBox, SampleEntry, TrakBox},
};
use std::fs::File;

/// トラック情報を格納する構造体
struct TrackInfo {
    media_type: String,
    duration: f64,
    codec: String,
    sample_count: Option<u32>,
    chunk_count: Option<u32>,
}

fn main() -> noargs::Result<()> {
    // Create `noargs::RawArgs` having the result of `std::env::args()`.
    let mut args = noargs::raw_args();

    // Set metadata for help.
    args.metadata_mut().app_name = env!("CARGO_PKG_NAME");
    args.metadata_mut().app_description = env!("CARGO_PKG_DESCRIPTION");

    // Handle well-known flags.
    if noargs::VERSION_FLAG.take(&mut args).is_present() {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if noargs::HELP_FLAG.take(&mut args).is_present() {
        args.metadata_mut().help_mode = true;
    }

    // Handle application specific args.
    let mp4_file: Option<String> = noargs::arg("mp4_file")
        .doc("The path to the MP4 file")
        .take(&mut args)
        .parse_if_present()?;

    // Check unexpected args and build help text if need.
    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    // Check if mp4_file is present.
    let file_path = match mp4_file {
        Some(file) => file,
        None => {
            println!("No mp4_file provided.");
            return Ok(());
        }
    };

    // ファイル読み込み
    let file = File::open(&file_path)?;
    let mp4_file: Mp4File<RootBox> = Mp4File::decode(file).unwrap();

    // MovieBox の情報を print する
    for box_item in mp4_file.boxes.iter() {
        if let RootBox::Moov(moov_box) = box_item {
            print_mp4_info(moov_box);
        }
    }

    Ok(())
}

/// 秒数から「分:秒」形式の文字列を生成する
fn format_duration(duration_seconds: f64) -> String {
    let minutes = (duration_seconds / 60.0).floor();
    let seconds = duration_seconds % 60.0;
    format!(
        "{:.0}分{:.1}秒 ({:.2}秒)",
        minutes, seconds, duration_seconds
    )
}

/// トラックから情報を抽出する
fn get_track_info(trak: &TrakBox) -> TrackInfo {
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
        Some(sample_entry) => get_codec_name(sample_entry),
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

fn print_mp4_info(moov_box: &MoovBox) {
    println!("MP4ファイル情報：");

    // ファイル全体の時間を計算して表示
    let movie_timescale = moov_box.mvhd_box.timescale.get() as f64;
    let movie_duration = moov_box.mvhd_box.duration as f64 / movie_timescale;
    println!("総再生時間: {}", format_duration(movie_duration));

    // トラック数を出力
    println!("トラック数: {}", moov_box.trak_boxes.len());

    // 各トラックの情報を出力
    for (i, trak) in moov_box.trak_boxes.iter().enumerate() {
        println!("\nトラック {}:", i + 1);

        // トラック情報を取得
        let track_info = get_track_info(trak);

        // 情報を表示
        println!("メディアタイプ: {}", track_info.media_type);
        println!("再生時間: {}", format_duration(track_info.duration));
        println!("コーデック: {}", track_info.codec);

        // サンプル情報があれば表示
        if let Some(sample_count) = track_info.sample_count {
            println!("サンプル数: {}", sample_count);
        }
        if let Some(chunk_count) = track_info.chunk_count {
            println!("チャンク数: {}", chunk_count);
        }
    }
}

fn get_codec_name(sample_entry: &SampleEntry) -> String {
    match sample_entry {
        SampleEntry::Avc1(_) => "AVC(H.264)".to_string(),
        SampleEntry::Hev1(_) => "HEVC(H.265)".to_string(),
        SampleEntry::Vp08(_) => "VP8".to_string(),
        SampleEntry::Vp09(_) => "VP9".to_string(),
        SampleEntry::Av01(_) => "AV1".to_string(),
        SampleEntry::Opus(_) => "Opus".to_string(),
        SampleEntry::Mp4a(_) => "MPEG AAC Audio (mp4a)".to_string(),
        SampleEntry::Unknown(unknown) => {
            let box_type = String::from_utf8_lossy(&unknown.box_type.as_bytes());
            format!("不明 ({})", box_type)
        }
    }
}
