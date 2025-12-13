//! 入出力の抽象化層
//!
//! stdin/stdout 対応と TTY 検出を提供する。

use std::fs::File;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;

/// 入力ソースの抽象化
#[derive(Debug)]
pub enum InputSource {
    /// ファイルからの入力
    File(PathBuf),
    /// 標準入力からの入力
    Stdin,
}

impl InputSource {
    /// 引数から入力ソースを決定する
    ///
    /// - `Some("-")` または `None`（stdin が TTY でない場合）→ Stdin
    /// - `Some(path)` → File
    /// - `None`（stdin が TTY の場合）→ None を返す（ヘルプ表示用）
    pub fn from_arg(arg: Option<String>) -> Option<Self> {
        match arg {
            Some(path) if path == "-" => Some(InputSource::Stdin),
            Some(path) => Some(InputSource::File(PathBuf::from(path))),
            None => {
                // 引数がない場合、stdin が TTY でなければ stdin を使用
                if !io::stdin().is_terminal() {
                    Some(InputSource::Stdin)
                } else {
                    // TTY の場合は None（ヘルプ表示用）
                    None
                }
            }
        }
    }

    /// 入力からデータを読み込む Reader を取得
    pub fn reader(&self) -> io::Result<Box<dyn Read>> {
        match self {
            InputSource::File(path) => Ok(Box::new(File::open(path)?)),
            InputSource::Stdin => Ok(Box::new(io::stdin().lock())),
        }
    }

    /// 入力ソースの説明を取得（エラーメッセージ用）
    pub fn description(&self) -> String {
        match self {
            InputSource::File(path) => path.display().to_string(),
            InputSource::Stdin => "stdin".to_string(),
        }
    }
}

/// 出力先の抽象化
#[derive(Debug)]
pub enum OutputSink {
    /// ファイルへの出力
    File(PathBuf),
    /// 標準出力への出力
    Stdout,
}

impl OutputSink {
    /// 引数から出力先を決定する
    ///
    /// - `Some("-")` → Stdout（TTY チェックあり）
    /// - `Some(path)` → File
    /// - `None` → Stdout（TTY チェックあり）
    ///
    /// stdout が TTY の場合はエラーを返す（バイナリ出力の防止）
    pub fn from_arg(arg: Option<String>, allow_tty: bool) -> Result<Self, String> {
        match arg {
            Some(path) if path == "-" => {
                if !allow_tty && io::stdout().is_terminal() {
                    return Err(
                        "stdout がターミナルです。ファイルまたはパイプにリダイレクトしてください"
                            .into(),
                    );
                }
                Ok(OutputSink::Stdout)
            }
            Some(path) => Ok(OutputSink::File(PathBuf::from(path))),
            None => {
                if !allow_tty && io::stdout().is_terminal() {
                    return Err(
                        "stdout がターミナルです。-o オプションで出力ファイルを指定するか、パイプにリダイレクトしてください"
                            .into(),
                    );
                }
                Ok(OutputSink::Stdout)
            }
        }
    }

    /// 出力用の Writer を取得
    pub fn writer(&self) -> io::Result<Box<dyn Write>> {
        match self {
            OutputSink::File(path) => Ok(Box::new(File::create(path)?)),
            OutputSink::Stdout => Ok(Box::new(io::stdout().lock())),
        }
    }

    /// 出力先の説明を取得（メッセージ用）
    pub fn description(&self) -> String {
        match self {
            OutputSink::File(path) => path.display().to_string(),
            OutputSink::Stdout => "stdout".to_string(),
        }
    }

    /// ファイル出力かどうか
    pub fn is_file(&self) -> bool {
        matches!(self, OutputSink::File(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_source_from_arg_with_file() {
        let source = InputSource::from_arg(Some("test.mp4".to_string()));
        assert!(matches!(source, Some(InputSource::File(_))));
    }

    #[test]
    fn test_input_source_from_arg_with_dash() {
        let source = InputSource::from_arg(Some("-".to_string()));
        assert!(matches!(source, Some(InputSource::Stdin)));
    }

    #[test]
    fn test_output_sink_from_arg_with_file() {
        let sink = OutputSink::from_arg(Some("output.mp4".to_string()), false);
        assert!(matches!(sink, Ok(OutputSink::File(_))));
    }
}
