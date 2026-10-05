//! Compiles translations, picks the app icon, and embeds the icon and version
//! information in Windows executables.

use std::path::PathBuf;

fn main() {
    fastframe_i18n::build::compile_catalogs("assets/i18n");

    // Fork: files in the gitignored `branding/` folder replace the icon, and
    // upstream's stays as the placeholder without them (FORK.md).
    println!("cargo:rerun-if-changed=branding");
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("set by Cargo"));
    let pick = |name: &str| Some(root.join("branding").join(name)).filter(|path| path.is_file());
    let custom = pick("icon.svg");
    let mark = custom
        .clone()
        .unwrap_or_else(|| root.join("packaging/icons/zapfast.svg"));
    let small = pick("icon-small.svg")
        .or_else(|| custom.clone())
        .unwrap_or_else(|| root.join("packaging/icons/zapfast-small.svg"));
    println!("cargo:rustc-env=ZAPFAST_ICON_SVG={}", mark.display());
    println!("cargo:rustc-env=ZAPFAST_ICON_SMALL_SVG={}", small.display());

    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=packaging/windows/zapfast.ico");
        let icon = pick("icon.ico")
            .or_else(|| custom.as_deref().and_then(drawn_icon))
            .unwrap_or_else(|| root.join("packaging/windows/zapfast.ico"));
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon(&icon.to_string_lossy())
            .set("ProductName", "ZapFast")
            .set("FileDescription", "ZapFast");
        if let Err(error) = resource.compile() {
            println!("cargo:warning=Windows resources not embedded: {error}");
        }
    }
}

/// Draws the branding SVG into an icon file for the executable, or warns and
/// leaves the placeholder in place.
#[cfg(windows)]
fn drawn_icon(svg: &std::path::Path) -> Option<PathBuf> {
    let out = PathBuf::from(std::env::var("OUT_DIR").ok()?).join("icon.ico");
    match write_icon(svg, &out) {
        Ok(()) => Some(out),
        Err(error) => {
            println!("cargo:warning=branding/icon.svg not used for the exe icon: {error}");
            None
        }
    }
}

/// Writes an ICO holding the SVG at the sizes Windows asks for, each image
/// stored as PNG, which Windows Vista and later read.
#[cfg(windows)]
fn write_icon(
    svg: &std::path::Path,
    out: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    const SIDES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];
    let tree = resvg::usvg::Tree::from_data(&std::fs::read(svg)?, &Default::default())?;
    let mut images = Vec::new();
    for side in SIDES {
        let mut pixmap = resvg::tiny_skia::Pixmap::new(side, side).ok_or("empty icon")?;
        let scale = side as f32 / tree.size().width();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );
        images.push((side, pixmap.encode_png()?));
    }
    // Header: reserved, type 1 (icon), image count.
    let mut ico = vec![0, 0, 1, 0];
    ico.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len() as u32;
    for (side, png) in &images {
        // A side of 0 stands for 256.
        let edge = if *side == 256 { 0 } else { *side as u8 };
        ico.extend_from_slice(&[edge, edge, 0, 0]);
        ico.extend_from_slice(&1u16.to_le_bytes()); // colour planes
        ico.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        ico.extend_from_slice(&(png.len() as u32).to_le_bytes());
        ico.extend_from_slice(&offset.to_le_bytes());
        offset += png.len() as u32;
    }
    for (_, png) in images {
        ico.extend(png);
    }
    std::fs::write(out, ico)?;
    Ok(())
}
