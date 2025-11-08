// 共通フラグ
const HELP_FLAG: noargs::FlagSpec = noargs::HELP_FLAG
    .doc("ヘルプメッセージを表示します ('--help' なら詳細、'-h' なら簡易版を表示)");
const VERSION_FLAG: noargs::FlagSpec = noargs::VERSION_FLAG.doc("バージョン情報を表示します");

// サブコマンド
const INFO_COMMAND: noargs::CmdSpec = noargs::cmd("info").doc("MP4 ファイルの情報を取得します");
const EXTRACT_COMMAND: noargs::CmdSpec =
    noargs::cmd("extract").doc("MP4 ファイルから指定秒数範囲を抽出します");

fn main() -> noargs::Result<()> {
    let mut args = noargs::raw_args();
    args.metadata_mut().app_name = env!("CARGO_PKG_NAME");
    args.metadata_mut().app_description = env!("CARGO_PKG_DESCRIPTION");

    // 共通系のフラグ処理
    HELP_FLAG.take_help(&mut args);

    if VERSION_FLAG.take(&mut args).is_present() {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // サブコマンドで分岐する
    if INFO_COMMAND.take(&mut args).is_present() {
        mp4util::subcommand_info::run(args)?;
    } else if EXTRACT_COMMAND.take(&mut args).is_present() {
        todo!();
    } else if let Some(help) = args.finish()? {
        print!("{help}");
    }

    Ok(())
}
