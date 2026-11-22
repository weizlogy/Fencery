use std::io::Write;

/// ロガーを初期化する。
///
/// ログレベルは環境変数 `RUST_LOG` で制御する（例: `DEBUG`, `INFO`）。
pub fn init() {
    env_logger::Builder::from_default_env()
        .format_timestamp_millis()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} <{}> {}",
                buf.timestamp(),
                record.level(),
                record.args()
            )
        })
        .init();
}

/// デバッグレベルのメッセージを出力する。
pub fn log_debug(msg: &str) {
    log::debug!("{}", msg);
}

/// 情報レベルのメッセージを出力する。
pub fn log_info(msg: &str) {
    log::info!("{}", msg);
}

/// 警告レベルのメッセージを出力する。
pub fn log_warn(msg: &str) {
    log::warn!("{}", msg);
}

/// エラーレベルのメッセージを出力する。
pub fn log_error(msg: &str) {
    log::error!("{}", msg);
}

#[cfg(test)]
mod tests {
    use super::*;

    // ロガーの初期化はプロセスあたり一度で十分である。
    fn ensure_logger_initialized() {
        static LOGGER_INIT: std::sync::Once = std::sync::Once::new();
        LOGGER_INIT.call_once(|| {
            init();
        });
    }

    #[test]
    fn test_log_functions_do_not_panic() {
        ensure_logger_initialized();

        log_debug("debug");
        log_info("info");
        log_warn("warn");
        log_error("error");
    }
}
