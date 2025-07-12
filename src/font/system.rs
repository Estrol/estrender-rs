// use crate::{dbg_log, font_::FontStyle};
use super::{
    FontInfo,
    FontStyle,
};

pub fn search_system_font() -> Vec<FontInfo> {
    // Determine system font directories based on OS
    let mut font_dirs = Vec::new();

    #[cfg(target_os = "windows")]
    {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
        font_dirs.push(format!("{}/Fonts", windir));
    }
    #[cfg(target_os = "linux")]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home".to_string());
        font_dirs.push(format!("{}/.fonts", home));
        font_dirs.push("/usr/share/fonts".to_string());
        font_dirs.push("/usr/local/share/fonts".to_string());
    }
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/Users".to_string());
        font_dirs.push(format!("{}/Library/Fonts", home));
        font_dirs.push("/Library/Fonts".to_string());
        font_dirs.push("/System/Library/Fonts".to_string());
    }

    let mut fonts = Vec::new();
    for font_dir in font_dirs {
        let path = std::path::Path::new(&font_dir);
        if path.exists() && path.is_dir() {
            if let Ok(entries) = path.read_dir() {
                for entry in entries.flatten() {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_file() {
                            let font_path = entry.path();
                            if let Some(ext) = font_path.extension() {
                                let ext = ext.to_str().unwrap_or("").to_lowercase();

                                if ext == "ttf" || ext == "otf" {
                                    if let Some(font_info) = get_font_info(&font_path) {
                                        fonts.push(font_info);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if fonts.is_empty() {
        crate::dbg_log!("No system fonts found.");
    } else {
        crate::dbg_log!("Found {} usable system fonts.", fonts.len());
    }

    fonts
}

pub fn get_font_info(path: &std::path::Path) -> Option<FontInfo> {
    let data = std::fs::read(path);
    if data.is_err() {
        crate::dbg_log!(
            "Failed to read font file at path: {}, {}",
            path.display(),
            data.err().unwrap()
        );
        return None;
    }

    let data = data.unwrap();

    let face = ttf_parser::Face::parse(&data, 0);
    if face.is_err() {
        crate::dbg_log!(
            "Failed to parse font file at path: {}, {}",
            path.display(),
            face.err().unwrap()
        );
        return None;
    }

    let face = face.unwrap();

    let font_family_name = face
        .names()
        .into_iter()
        .find(|name| {
            name.name_id == ttf_parser::name_id::FAMILY
                || name.name_id == ttf_parser::name_id::SUBFAMILY
                || name.name_id == ttf_parser::name_id::FULL_NAME
        })
        .and_then(|name| name.to_string());

    if font_family_name.is_none() {
        return None;
    }

    let font_family_name = font_family_name.unwrap();
    let mut font_style = FontStyle::empty();

    if face.is_bold() {
        font_style |= FontStyle::BOLD;
    }

    if face.is_italic() {
        font_style |= FontStyle::ITALIC;
    }

    let font_info = FontInfo {
        name: font_family_name,
        path: path.to_path_buf(),
        style: font_style,
    };

    Some(font_info)
}
