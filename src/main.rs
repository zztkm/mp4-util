use noargs;
use std::fs::File;
use shiguredo_mp4::{
    boxes::{
        RootBox, MoovBox, SampleEntry
    },
    Decode, Mp4File
};

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
    let mp4_file: Option<String>= noargs::arg("mp4_file")
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

    // トラックごとのコーデック情報を抽出
    for box_item in mp4_file.boxes.iter() {
        if let RootBox::Moov(moov_box) = box_item {
            print_codec_info(moov_box);
        }
    }

    Ok(())
}

fn print_codec_info(moov_box: &MoovBox) {
    println!("MP4ファイル情報：");
    
    // トラック数を出力
    println!("トラック数: {}", moov_box.trak_boxes.len());
    
    // 各トラックの情報を出力
    for (i, trak) in moov_box.trak_boxes.iter().enumerate() {
        println!("\nトラック {}:", i + 1);
        
        // メディアタイプ (ビデオ/オーディオ)
        let handler_type = &trak.mdia_box.hdlr_box.handler_type;
        let media_type = match handler_type {
            b"vide" => "ビデオ",
            b"soun" => "オーディオ",
            b"hint" => "ヒント",
            b"meta" => "メタデータ",
            _ => "不明"
        };
        println!("メディアタイプ: {}", media_type);
        
        // サンプルエントリからコーデック情報を取得
        if let Some(sample_entry) = trak.mdia_box.minf_box.stbl_box.stsd_box.entries.first() {
            match sample_entry {
                SampleEntry::Avc1(_) => println!("コーデック: AVC/H.264"),
                SampleEntry::Hev1(_) => println!("コーデック: HEVC/H.265"),
                SampleEntry::Vp08(_) => println!("コーデック: VP8"),
                SampleEntry::Vp09(_) => println!("コーデック: VP9"),
                SampleEntry::Av01(_) => println!("コーデック: AV1"),
                SampleEntry::Opus(_) => println!("コーデック: Opus"),
                SampleEntry::Mp4a(_) => println!("コーデック: AAC"),
                SampleEntry::Unknown(unknown) => println!("コーデック: 不明 ({})", String::from_utf8_lossy(&unknown.box_type.as_bytes())),
            }
        } else {
            println!("コーデック: 不明 (サンプルエントリなし)");
        }
        
        // 追加情報（解像度、サンプルレートなど）は必要に応じて追加できます
    }
}
