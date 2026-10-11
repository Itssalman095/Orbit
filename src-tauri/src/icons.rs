//! Windows icon extraction: turns .exe/.dll/.lnk targets and image files
//! into PNG data URLs for the webview.
//!
//! Strategy (in order):
//!  1. Image files (png/jpg/jpeg/bmp/ico) are decoded directly.
//!  2. IShellItemImageFactory::GetImage — high quality, resolves .lnk icons.
//!  3. SHGetFileInfoW as a fallback for older/icon-less shells.
//!
//! Extracted PNGs are cached on disk keyed by (path, mtime) so repeated
//! loads are instant. COM is initialized on a dedicated STA thread per
//! extraction (extraction is rare and must never touch the main thread).

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Cursor;
use std::path::Path;

use base64::Engine;

pub fn png_data_url(png: &[u8]) -> String {
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    )
}

/// Returns a PNG data URL for the given file, or None if extraction fails.
/// `cache_dir` holds extracted icons between runs.
pub fn get_icon_data_url(path: &str, cache_dir: &Path) -> Option<String> {
    let src = Path::new(path);
    let meta = src.metadata().ok()?;
    if !meta.is_file() {
        return None;
    }
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let mut h = DefaultHasher::new();
    path.hash(&mut h);
    mtime.hash(&mut h);
    let key = h.finish();

    let cache_file = cache_dir.join(format!("icon-{key:016x}.png"));
    if let Ok(bytes) = fs::read(&cache_file) {
        return Some(png_data_url(&bytes));
    }

    let png = extract_png(path)?;
    if fs::create_dir_all(cache_dir).is_ok() {
        let _ = fs::write(&cache_file, &png);
    }
    Some(png_data_url(&png))
}

fn extract_png(path: &str) -> Option<Vec<u8>> {
    let lower = path.to_ascii_lowercase();
    let ext = Path::new(&lower)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_string();

    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "bmp" | "ico" => decode_image_file(path),
        _ => extract_shell_icon(path),
    }
}

// ---------------------------------------------------------------------------
// Direct image decoding
// ---------------------------------------------------------------------------

fn decode_image_file(path: &str) -> Option<Vec<u8>> {
    let img = image::ImageReader::open(path).ok()?.decode().ok()?;
    let img = resize_fit(&img, 64);
    let mut out = Vec::new();
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
        .ok()?;
    Some(out)
}

fn resize_fit(img: &image::DynamicImage, max: u32) -> image::RgbaImage {
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let longest = w.max(h);
    if longest <= max {
        return rgba;
    }
    let nw = ((w as f64 / longest as f64) * max as f64).round().max(1.0) as u32;
    let nh = ((h as f64 / longest as f64) * max as f64).round().max(1.0) as u32;
    image::imageops::resize(&rgba, nw, nh, image::imageops::FilterType::Triangle)
}

// ---------------------------------------------------------------------------
// Shell icon extraction (Windows)
// ---------------------------------------------------------------------------

#[cfg(windows)]
fn extract_shell_icon(path: &str) -> Option<Vec<u8>> {
    let owned = path.to_string();
    let handle = std::thread::Builder::new()
        .name("orbit-icon-extract".into())
        .spawn(move || unsafe { com_extract(&owned) })
        .ok()?;
    handle.join().ok().flatten()
}

#[cfg(windows)]
unsafe fn com_extract(path: &str) -> Option<Vec<u8>> {
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};

    let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let com_ready = hr.is_ok();
    let result = unsafe {
        let via_factory = via_item_image_factory(path);
        via_factory.or_else(|| via_shgetfileinfo(path))
    };
    if com_ready {
        unsafe { windows::Win32::System::Com::CoUninitialize() };
    }
    result
}

#[cfg(windows)]
unsafe fn via_item_image_factory(path: &str) -> Option<Vec<u8>> {
    use windows::core::Interface;
    use windows::core::HSTRING;
    use windows::Win32::Foundation::SIZE;
    use windows::Win32::Graphics::Gdi::HGDIOBJ;
    use windows::Win32::System::Com::IBindCtx;
    use windows::Win32::UI::Shell::IShellItem;
    use windows::Win32::UI::Shell::{
        IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_ICONONLY,
    };

    unsafe {
        let wide = HSTRING::from(path);
        let item: IShellItem = SHCreateItemFromParsingName(&wide, None::<&IBindCtx>)
            .ok()
            .or(None)?;
        let factory: IShellItemImageFactory = item.cast().ok()?;

        let hbmp = factory
            .GetImage(SIZE { cx: 64, cy: 64 }, SIIGBF_ICONONLY)
            .ok()?;

        let png = hbitmap_to_png(hbmp);
        let _ = windows::Win32::Graphics::Gdi::DeleteObject(HGDIOBJ(hbmp.0));
        png
    }
}

#[cfg(windows)]
unsafe fn via_shgetfileinfo(path: &str) -> Option<Vec<u8>> {
    use windows::core::HSTRING;
    use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
    use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
    use windows::Win32::UI::WindowsAndMessaging::DestroyIcon;

    unsafe {
        let wide = HSTRING::from(path);
        let mut sfi = SHFILEINFOW::default();
        let ok = SHGetFileInfoW(
            &wide,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut sfi),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        );
        if ok == 0 || sfi.hIcon.is_invalid() {
            return None;
        }
        let png = hicon_to_png(sfi.hIcon);
        let _ = DestroyIcon(sfi.hIcon);
        png
    }
}

/// HICON -> PNG through GetIconInfo + GetDIBits.
#[cfg(windows)]
unsafe fn hicon_to_png(hicon: windows::Win32::UI::WindowsAndMessaging::HICON) -> Option<Vec<u8>> {
    use windows::Win32::Graphics::Gdi::DeleteObject;
    use windows::Win32::Graphics::Gdi::HGDIOBJ;
    use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};

    unsafe {
        let mut info = ICONINFO::default();
        GetIconInfo(hicon, &mut info).ok()?;
        let png = if !info.hbmColor.is_invalid() {
            hbitmap_to_png(info.hbmColor)
        } else {
            None
        };
        let _ = DeleteObject(HGDIOBJ(info.hbmColor.0));
        let _ = DeleteObject(HGDIOBJ(info.hbmMask.0));
        png
    }
}

/// HBITMAP (32bpp) -> PNG. Handles BGRA order and opaque bitmaps.
#[cfg(windows)]
unsafe fn hbitmap_to_png(hbmp: windows::Win32::Graphics::Gdi::HBITMAP) -> Option<Vec<u8>> {
    use windows::Win32::Graphics::Gdi::{
        GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
        DIB_RGB_COLORS, HDC, HGDIOBJ,
    };

    unsafe {
        let mut bmp = BITMAP::default();
        let got = GetObjectW(
            HGDIOBJ(hbmp.0),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bmp as *mut BITMAP as *mut core::ffi::c_void),
        );
        if got == 0 {
            return None;
        }
        let w = bmp.bmWidth.max(1);
        let h = bmp.bmHeight.abs().max(1);

        let hdc: HDC = GetDC(None);
        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = w;
        bmi.bmiHeader.biHeight = -h; // top-down
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB.0;

        let mut buf = vec![0u8; (w as usize) * (h as usize) * 4];
        let lines = GetDIBits(
            hdc,
            hbmp,
            0,
            h as u32,
            Some(buf.as_mut_ptr() as *mut core::ffi::c_void),
            &mut bmi,
            DIB_RGB_COLORS,
        );
        let _ = ReleaseDC(None, hdc);
        if lines == 0 {
            return None;
        }

        // BGRA -> RGBA
        for px in buf.chunks_exact_mut(4) {
            px.swap(0, 2);
        }
        // Some bitmaps have no alpha channel (all zero): make them opaque.
        let has_alpha = buf.chunks_exact(4).any(|px| px[3] != 0);
        if !has_alpha {
            for px in buf.chunks_exact_mut(4) {
                px[3] = 255;
            }
        }

        let img = image::RgbaImage::from_raw(w as u32, h as u32, buf)?;
        let mut out = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
            .ok()?;
        Some(out)
    }
}

#[cfg(not(windows))]
fn extract_shell_icon(_path: &str) -> Option<Vec<u8>> {
    None
}

/// Extracts icons for all configured apps in the background and fills the
/// in-memory cache. Runs off-thread: COM extraction can take tens of ms.
pub fn warm_cache(state: std::sync::Arc<crate::state::SharedState>) {
    std::thread::spawn(move || {
        let cfg = state.config_snapshot();
        for app in &cfg.applications {
            let source = app
                .icon_path
                .clone()
                .unwrap_or_else(|| app.executable_path.clone());
            if state.icon_for(&source).is_some() {
                continue;
            }
            if let Some(url) = get_icon_data_url(&source, &state.paths.icon_cache_dir) {
                state.insert_icon(source, url);
            }
        }
        crate::logger::info("icon cache warmed");
    });
}
