use std::{
    fs,
    io,
    path::PathBuf,
};
use super::models::Settings;

/// 設定ファイルの保存先ディレクトリを解決する。
///
/// `%APPDATA%/Fencery` を標準とし、取得できない場合は実行ファイルのディレクトリを使用する。
fn get_settings_dir() -> io::Result<PathBuf> {
    let mut path = if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata)
    } else {
        let exe_path = std::env::current_exe()?;
        exe_path.parent().unwrap().to_path_buf()
    };

    path.push("Fencery");

    // 初回起動時にディレクトリを作成する。
    if !path.exists() {
        fs::create_dir_all(&path)?;
    }

    Ok(path)
}

/// `config.toml` へのフルパスを取得する。
pub fn get_config_path() -> io::Result<PathBuf> {
    Ok(get_settings_dir()?.join("config.toml"))
}

/// 設定ファイルを読み込む。
///
/// ファイルが存在しない場合は既定値を返す。破損時はファイルを退避しエラーを返す。
pub fn load_settings() -> Result<Settings, String> {
    let config_path = get_config_path().map_err(|e| e.to_string())?;

    if !config_path.exists() {
        log::info!("Config file not found. Using default settings.");
        return Ok(Settings::default());
    }

    let contents = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    let mut settings: Settings = match toml::from_str(&contents) {
        Ok(s) => s,
        Err(e) => {
            let bad_path = config_path.with_extension("toml.bad");
            log::error!("Failed to parse config file: {}. Backing up to {:?}", e, bad_path);
            let _ = fs::rename(&config_path, &bad_path); // 破損ファイルを `.bad` へ退避する。
            return Err(format!("Settings corruption detected. Original file saved as .bad"));
        }
    };

    // 読み込んだ値を検証・補正する。
    settings.validate();

    Ok(settings)
}

/// 設定ファイルをアトミックに保存する。
///
/// 一時ファイルへ書き出してからリネームすることで、途中終了による破損を防ぐ。
pub fn save_settings(settings: &Settings) -> Result<(), String> {
    let config_path = get_config_path().map_err(|e| e.to_string())?;
    let tmp_path = config_path.with_extension("tmp");

    let toml_string = toml::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    // 一時ファイルへ書き出す。
    fs::write(&tmp_path, toml_string)
        .map_err(|e| format!("Failed to write temporary config file: {}", e))?;

    // Windows の `fs::rename` は同一ボリューム内でアトミックに置換される。
    fs::rename(&tmp_path, &config_path)
        .map_err(|e| {
            // 失敗時は一時ファイルを削除して残骸を残さない。
            let _ = fs::remove_file(&tmp_path);
            format!("Failed to finalize config file save (rename error): {}", e)
        })?;

    log::debug!("Settings saved atomically to {:?}", config_path);
    Ok(())
}
