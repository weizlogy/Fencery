use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory1, IDWriteFontCollection, DWRITE_FACTORY_TYPE_SHARED,
};

/// DirectWrite ファクトリを作成する。
pub fn create_factory() -> Result<IDWriteFactory1, windows::core::Error> {
    unsafe {
        DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)
    }
}

/// インストール済みのフォントファミリ名を列挙する。
///
/// `config.toml` の `font_family` は "Meiryo" などの英語名で保存されるため、
/// ロケール名は en-US、ja-JP の順に優先し、見つからなければ先頭の名前を使用する。
pub fn get_installed_font_families(
    factory: &IDWriteFactory1,
) -> Result<Vec<String>, windows::core::Error> {
    unsafe {
        let mut collection: Option<IDWriteFontCollection> = None;
        factory.GetSystemFontCollection(&mut collection, false)?;
        let collection = collection.ok_or_else(|| windows::core::Error::from_win32())?;

        let mut families = Vec::new();
        let count = collection.GetFontFamilyCount();
        for i in 0..count {
            let family = collection.GetFontFamily(i)?;
            let names = family.GetFamilyNames()?;
            if names.GetCount() == 0 {
                continue;
            }

            // 保存済みの値との一致を優先するため英語名を選ぶ。
            let mut index = 0u32;
            let mut exists = windows::Win32::Foundation::BOOL(0);
            let _ = names.FindLocaleName(windows::core::w!("en-US"), &mut index, &mut exists);
            if !exists.as_bool() {
                let _ = names.FindLocaleName(windows::core::w!("ja-JP"), &mut index, &mut exists);
                if !exists.as_bool() {
                    index = 0;
                }
            }

            let len = names.GetStringLength(index)? as usize;
            let mut buf = vec![0u16; len + 1]; // 終端 null の分を確保する。
            names.GetString(index, &mut buf)?;
            let name = String::from_utf16_lossy(&buf[..len]);
            if !name.is_empty() {
                families.push(name);
            }
        }

        families.sort();
        families.dedup();
        Ok(families)
    }
}
