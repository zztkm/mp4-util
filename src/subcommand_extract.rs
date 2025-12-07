use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    num::NonZeroU32,
    path::PathBuf,
};

use shiguredo_mp4::{
    Decode, Mp4File, TrackKind,
    aux::SampleTableAccessor,
    boxes::{RootBox, SampleEntry, TrakBox},
    mux::{Mp4FileMuxer, Mp4FileMuxerOptions, Sample, estimate_maximum_moov_box_size},
};

const START_OPT: noargs::OptSpec = noargs::opt("start")
    .short('s')
    .doc("開始秒数")
    .ty("SECONDS")
    .example("10.0");

const END_OPT: noargs::OptSpec = noargs::opt("end")
    .short('e')
    .doc("終了秒数")
    .ty("SECONDS")
    .example("30.0");

const OUTPUT_OPT: noargs::OptSpec = noargs::opt("output")
    .short('o')
    .doc("出力ファイルパス")
    .ty("PATH")
    .example("output.mp4");

pub fn run(mut args: noargs::RawArgs) -> noargs::Result<()> {
    let input_file_path: PathBuf = noargs::arg("INPUT_FILE")
        .example("/path/to/input.mp4")
        .doc("抽出元の MP4 ファイル")
        .take(&mut args)
        .then(|a| a.value().parse())?;

    let start_sec: f64 = START_OPT.take(&mut args).then(|o| o.value().parse())?;

    let end_sec: f64 = END_OPT.take(&mut args).then(|o| o.value().parse())?;

    let output_file_path: PathBuf = OUTPUT_OPT.take(&mut args).then(|o| o.value().parse())?;

    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    // 入力の検証
    if start_sec < 0.0 {
        return Err("開始秒数は0以上である必要があります".into());
    }
    if end_sec <= start_sec {
        return Err("終了秒数は開始秒数より大きい必要があります".into());
    }

    // MP4 ファイルを読み込み
    let mut file = File::open(&input_file_path)?;
    let mut file_data = Vec::new();
    file.read_to_end(&mut file_data)?;

    let (mp4_file, _) = Mp4File::decode(&file_data)
        .map_err(|e| format!("MP4 ファイルの解析に失敗しました: {}", e))?;

    // moov ボックスを取得
    let moov_box = mp4_file
        .boxes
        .iter()
        .find_map(|box_item| {
            if let RootBox::Moov(moov_box) = box_item {
                Some(moov_box)
            } else {
                None
            }
        })
        .ok_or("moov ボックスが見つかりません")?;

    // トラック情報を収集
    let mut track_infos: Vec<TrackExtractInfo> = Vec::new();
    for trak in &moov_box.trak_boxes {
        let handler_type = &trak.mdia_box.hdlr_box.handler_type;
        let track_kind = match handler_type {
            b"vide" => TrackKind::Video,
            b"soun" => TrackKind::Audio,
            _ => continue, // ビデオ・オーディオ以外はスキップ
        };

        let timescale = trak.mdia_box.mdhd_box.timescale.get();
        let sample_table = SampleTableAccessor::new(&trak.mdia_box.minf_box.stbl_box)
            .map_err(|e| format!("サンプルテーブルの解析に失敗しました: {}", e))?;

        // 開始・終了タイムスタンプを計算
        let start_timestamp = (start_sec * timescale as f64) as u64;
        let end_timestamp = (end_sec * timescale as f64) as u64;

        // 開始サンプルを見つける（キーフレーム境界に調整）
        let start_sample = sample_table
            .get_sample_by_timestamp(start_timestamp)
            .ok_or("指定された開始時間にサンプルが見つかりません")?;

        // ビデオトラックの場合はキーフレームに調整
        let actual_start_sample = if track_kind == TrackKind::Video {
            start_sample
                .sync_sample()
                .ok_or("開始位置より前にキーフレームが見つかりません")?
        } else {
            start_sample
        };

        // 終了サンプルを見つける
        let end_sample = sample_table
            .get_sample_by_timestamp(end_timestamp)
            .or_else(|| {
                // 終了時間がファイル末尾を超えている場合は最後のサンプルを使用
                let sample_count = sample_table.sample_count();
                sample_table.get_sample(NonZeroU32::new(sample_count)?)
            })
            .ok_or("指定された終了時間にサンプルが見つかりません")?;

        // サンプルエントリーを取得
        let sample_entry = actual_start_sample.chunk().sample_entry().clone();

        track_infos.push(TrackExtractInfo {
            track_kind,
            timescale: NonZeroU32::new(timescale).unwrap(),
            sample_entry,
            start_sample_index: actual_start_sample.index(),
            end_sample_index: end_sample.index(),
            start_timestamp: actual_start_sample.timestamp(),
            trak_box: trak.clone(),
        });
    }

    if track_infos.is_empty() {
        return Err("ビデオまたはオーディオトラックが見つかりません".into());
    }

    // サンプル数を見積もって moov ボックスサイズを予約
    let sample_counts: Vec<usize> = track_infos
        .iter()
        .map(|t| (t.end_sample_index.get() - t.start_sample_index.get() + 1) as usize)
        .collect();
    let reserved_moov_size = estimate_maximum_moov_box_size(&sample_counts);

    // Muxer を初期化
    let options = Mp4FileMuxerOptions {
        reserved_moov_box_size: reserved_moov_size,
        ..Default::default()
    };
    let mut muxer = Mp4FileMuxer::with_options(options)
        .map_err(|e| format!("Muxer の初期化に失敗しました: {}", e))?;

    // 出力ファイルを作成
    let mut output_file = File::create(&output_file_path)?;

    // 初期ボックスを書き込み
    let initial_bytes = muxer.initial_boxes_bytes();
    output_file.write_all(initial_bytes)?;
    let mut current_offset = initial_bytes.len() as u64;

    // 各トラックからサンプルを抽出して書き込み
    // トラックごとにサンプルを時系列順で処理
    let mut sample_iterators: Vec<SampleIterator> = track_infos
        .iter()
        .map(|info| {
            let sample_table = SampleTableAccessor::new(&info.trak_box.mdia_box.minf_box.stbl_box)
                .expect("already validated");
            SampleIterator {
                track_info: info,
                sample_table,
                current_index: info.start_sample_index,
                base_timestamp: info.start_timestamp,
                is_first_sample: true,
            }
        })
        .collect();

    // 全てのトラックのサンプルを時系列順にインターリーブ
    loop {
        // 次のサンプルを持つトラックを見つける（タイムスタンプが最小のもの）
        let mut next_track_idx = None;
        let mut min_timestamp = u64::MAX;

        for (idx, iter) in sample_iterators.iter().enumerate() {
            if iter.current_index <= iter.track_info.end_sample_index {
                let sample = iter
                    .sample_table
                    .get_sample(iter.current_index)
                    .expect("valid index");
                let normalized_timestamp = normalize_timestamp(
                    sample.timestamp() - iter.base_timestamp,
                    iter.track_info.timescale.get(),
                );
                if normalized_timestamp < min_timestamp {
                    min_timestamp = normalized_timestamp;
                    next_track_idx = Some(idx);
                }
            }
        }

        let Some(track_idx) = next_track_idx else {
            break; // 全てのサンプルを処理完了
        };

        let iter = &mut sample_iterators[track_idx];
        let sample_accessor = iter
            .sample_table
            .get_sample(iter.current_index)
            .expect("valid index");

        // サンプルデータを読み取り
        let data_offset = sample_accessor.data_offset() as usize;
        let data_size = sample_accessor.data_size() as usize;
        let sample_data = &file_data[data_offset..data_offset + data_size];

        // 出力ファイルに書き込み
        output_file.write_all(sample_data)?;

        // Muxer にサンプルを追加
        let sample = Sample {
            track_kind: iter.track_info.track_kind,
            sample_entry: if iter.is_first_sample {
                Some(iter.track_info.sample_entry.clone())
            } else {
                None
            },
            keyframe: sample_accessor.is_sync_sample(),
            timescale: iter.track_info.timescale,
            duration: sample_accessor.duration(),
            data_offset: current_offset,
            data_size,
        };
        muxer
            .append_sample(&sample)
            .map_err(|e| format!("サンプルの追加に失敗しました: {}", e))?;

        current_offset += data_size as u64;
        iter.current_index = iter.current_index.saturating_add(1);
        iter.is_first_sample = false;
    }

    // ファイナライズ
    let finalized = muxer
        .finalize()
        .map_err(|e| format!("ファイナライズに失敗しました: {}", e))?;

    // ファイナライズ後のボックス情報をファイルに書き込み
    for (offset, bytes) in finalized.offset_and_bytes_pairs() {
        output_file.seek(SeekFrom::Start(offset))?;
        output_file.write_all(bytes)?;
    }

    // 結果を表示
    let video_info = track_infos
        .iter()
        .find(|t| t.track_kind == TrackKind::Video);
    let audio_info = track_infos
        .iter()
        .find(|t| t.track_kind == TrackKind::Audio);

    println!("抽出が完了しました: {}", output_file_path.display());
    if let Some(info) = video_info {
        let start_time = info.start_timestamp as f64 / info.timescale.get() as f64;
        let sample_count = info.end_sample_index.get() - info.start_sample_index.get() + 1;
        println!(
            "  ビデオ: {} サンプル (実際の開始時間: {:.3}秒)",
            sample_count, start_time
        );
    }
    if let Some(info) = audio_info {
        let sample_count = info.end_sample_index.get() - info.start_sample_index.get() + 1;
        println!("  オーディオ: {} サンプル", sample_count);
    }
    if finalized.is_faststart_enabled() {
        println!("  faststart: 有効");
    }

    Ok(())
}

/// トラック抽出情報
struct TrackExtractInfo {
    track_kind: TrackKind,
    timescale: NonZeroU32,
    sample_entry: SampleEntry,
    start_sample_index: NonZeroU32,
    end_sample_index: NonZeroU32,
    start_timestamp: u64,
    trak_box: TrakBox,
}

/// サンプルイテレーター
struct SampleIterator<'a> {
    track_info: &'a TrackExtractInfo,
    sample_table: SampleTableAccessor<&'a shiguredo_mp4::boxes::StblBox>,
    current_index: NonZeroU32,
    base_timestamp: u64,
    is_first_sample: bool,
}

/// タイムスタンプを正規化（ナノ秒単位に変換）
fn normalize_timestamp(timestamp: u64, timescale: u32) -> u64 {
    timestamp * 1_000_000_000 / timescale as u64
}
