use std::{fs::File, path::PathBuf};

use crate::mp4::InputMp4;

pub fn run(mut args: noargs::RawArgs) -> noargs::Result<()> {
    let input_file_path: PathBuf = noargs::arg("INPUT_FILE")
        .example("/path/to/input.mp4")
        .doc("情報を取得する MP4 ファイル")
        .take(&mut args)
        .then(|a| a.value().parse())?;
    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    let file = File::open(input_file_path)?;

    let input_mp4 = InputMp4::parse(file)?;
    print_mp4_info(&input_mp4);
    Ok(())
}

fn print_mp4_info(mp4: &InputMp4) {
    let tracks = match mp4.get_track_infos() {
        Some(tracks) => tracks,
        None => {
            println!("トラック情報が取得できませんでした。");
            return;
        }
    };

    println!("MP4ファイル情報：");
    println!("トラック数: {}", tracks.len());

    for (i, track) in tracks.iter().enumerate() {
        println!("\nトラック {}:", i + 1);
        println!("メディアタイプ: {}", track.media_type);
        println!("再生時間: {}", format_duration(track.duration));
        println!("コーデック: {}", track.codec);

        if let Some(sample_count) = track.sample_count {
            println!("サンプル数: {}", sample_count);
        }
        if let Some(chunk_count) = track.chunk_count {
            println!("チャンク数: {}", chunk_count);
        }
    }
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
