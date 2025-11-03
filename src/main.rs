use std::fs::File;

use mp4util::mp4::InputMp4;

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
        .present_and_then(|a| a.value().parse())?;

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

    let file = File::open(file_path)?;

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
