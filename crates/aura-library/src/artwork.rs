//! 封面缩略图缓存（§29/§30/§66）：64/256 双尺寸、确定性 box-filter 缩放、
//! 临时文件 + rename 原子写、损坏缓存删除即可重建、解码限内存防 decompression
//! abuse。缓存只写应用数据目录（调用方传入），绝不触碰音乐源目录（§30）。

use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder, ImageReader, Limits};

use crate::error::{LibraryError, Result};

/// 缓存尺寸（§29；UI 需要时可调整）。
pub const THUMB_SIZES: &[u32] = &[64, 256];

/// 原始图字节数上限（§66）。
pub const MAX_SOURCE_BYTES: usize = 16 << 20;

/// 解码内存上限（像素×4字节 ≈ 16000×16000 RGBA）。
const MAX_DECODE_PIXELS: u32 = 16_000;

/// 缩略图缓存。
#[derive(Debug, Clone)]
pub struct ThumbnailCache {
    dir: PathBuf,
}

impl ThumbnailCache {
    #[must_use]
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// 目标文件路径（不保证存在）。
    #[must_use]
    pub fn path_for(&self, content_hash: &str, size: u32) -> PathBuf {
        self.dir.join(format!("{content_hash}_{size}.png"))
    }

    /// 取缩略图；不存在则生成。返回 PNG 文件路径。
    ///
    /// 失败只影响封面展示（`artwork_unavailable`），不影响库（§31）。
    ///
    /// # Errors
    /// 解码失败/超限/写盘失败 → [`LibraryError`]。
    pub fn get_or_create(&self, content_hash: &str, data: &[u8], size: u32) -> Result<PathBuf> {
        let target = self.path_for(content_hash, size);
        if target.is_file() {
            return Ok(target);
        }
        let png = render_thumbnail_png(data, size)?;
        atomic_write(&target, &png).map_err(|e| LibraryError::Io {
            path: Some(target.clone()),
            source: e,
        })?;
        Ok(target)
    }

    /// 删除某封面的全部尺寸缓存（损坏恢复入口：删后下次 get_or_create 重建）。
    ///
    /// # Errors
    /// 删除失败（非 NotFound）时返回 [`LibraryError::Io`]。
    pub fn invalidate(&self, content_hash: &str) -> Result<()> {
        for size in THUMB_SIZES {
            match std::fs::remove_file(self.path_for(content_hash, *size)) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => {
                    return Err(LibraryError::Io {
                        path: Some(self.path_for(content_hash, *size)),
                        source: e,
                    })
                }
            }
        }
        Ok(())
    }
}

/// 解码（受限）→ box-filter 缩放 → PNG 编码。纯函数，确定性。
///
/// # Errors
/// 字节超限/非图片/解码失败/维度超限 →
/// [`LibraryError::Metadata`]（category: `ArtworkUnavailable` 语义）。
pub fn render_thumbnail_png(data: &[u8], size: u32) -> Result<Vec<u8>> {
    if data.len() > MAX_SOURCE_BYTES {
        return Err(artwork_error("artwork too large"));
    }
    if data.len() < 8 {
        return Err(artwork_error("artwork too small"));
    }

    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DECODE_PIXELS);
    limits.max_image_height = Some(MAX_DECODE_PIXELS);
    let mut reader = ImageReader::new(Cursor::new(data));
    reader.limits(limits.clone());
    reader = reader
        .with_guessed_format()
        .map_err(|_| artwork_error("format probe failed"))?;
    let img = reader
        .decode()
        .map_err(|_| artwork_error("decode failed"))?;

    let rgba = img.to_rgba8();
    let thumb = resize_box(&rgba, size);

    let mut out = Vec::new();
    let encoder = PngEncoder::new(Cursor::new(&mut out));
    encoder
        .write_image(
            thumb.as_raw(),
            thumb.width(),
            thumb.height(),
            ExtendedColorType::Rgba8,
        )
        .map_err(|_| artwork_error("png encode failed"))?;
    Ok(out)
}

fn artwork_error(reason: &'static str) -> LibraryError {
    LibraryError::Metadata {
        path: PathBuf::from("<artwork>"),
        reason: reason.to_string(),
    }
}

/// 确定性 box-filter 缩放：最长边贴合 `target`（不放大）。
#[must_use]
pub fn resize_box(src: &image::RgbaImage, target: u32) -> image::RgbaImage {
    let (sw, sh) = (src.width(), src.height());
    let scale = f64::from(target) / f64::from(sw.max(sh).max(1));
    if scale >= 1.0 {
        // 不放大：原图即缩略。
        return src.clone();
    }
    let dw = u32::max(1, (f64::from(sw) * scale).round() as u32);
    let dh = u32::max(1, (f64::from(sh) * scale).round() as u32);
    let mut out = image::RgbaImage::new(dw, dh);
    for oy in 0..dh {
        let y0 = (u64::from(oy) * u64::from(sh)) / u64::from(dh);
        let y1 = u64::max(y0 + 1, (u64::from(oy + 1) * u64::from(sh)) / u64::from(dh));
        for ox in 0..dw {
            let x0 = (u64::from(ox) * u64::from(sw)) / u64::from(dw);
            let x1 = u64::max(x0 + 1, (u64::from(ox + 1) * u64::from(sw)) / u64::from(dw));
            let (mut r, mut g, mut b, mut a) = (0u64, 0u64, 0u64, 0u64);
            let mut n = 0u64;
            for y in y0..y1.min(u64::from(sh)) {
                for x in x0..x1.min(u64::from(sw)) {
                    let px =
                        src.get_pixel(u32::try_from(x).unwrap_or(0), u32::try_from(y).unwrap_or(0));
                    let [pr, pg, pb, pa] = px.0;
                    r += u64::from(pr);
                    g += u64::from(pg);
                    b += u64::from(pb);
                    a += u64::from(pa);
                    n += 1;
                }
            }
            if n == 0 {
                n = 1;
            }
            out.put_pixel(
                ox,
                oy,
                image::Rgba([
                    u8::try_from(r / n).unwrap_or(255),
                    u8::try_from(g / n).unwrap_or(255),
                    u8::try_from(b / n).unwrap_or(255),
                    u8::try_from(a / n).unwrap_or(255),
                ]),
            );
        }
    }
    out
}

/// 临时文件 + rename 原子写（§29）。
fn atomic_write(target: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = target.with_extension("png.tmp");
    std::fs::write(&tmp, bytes)?;
    // rename 同盘原子；残留 tmp 由下次覆盖。
    std::fs::rename(&tmp, target)
}

/// 恶意/异常输入不得 panic 的冒烟集（供测试复用）。
#[must_use]
pub fn is_probable_image(data: &[u8]) -> bool {
    crate::metadata::sniff_mime(data).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_png(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbaImage::from_fn(w, h, |x, y| {
            image::Rgba([
                u8::try_from(x * 7 % 256).unwrap(),
                u8::try_from(y * 13 % 256).unwrap(),
                128,
                255,
            ])
        });
        let mut out = Vec::new();
        PngEncoder::new(Cursor::new(&mut out))
            .write_image(img.as_raw(), w, h, ExtendedColorType::Rgba8)
            .unwrap();
        out
    }

    fn tiny_jpeg(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbaImage::from_fn(w, h, |x, y| {
            image::Rgba([
                u8::try_from(x * 3 % 256).unwrap(),
                64,
                u8::try_from(y * 5 % 256).unwrap(),
                255,
            ])
        });
        let mut buf = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut buf, image::ImageFormat::Jpeg)
            .unwrap();
        buf.into_inner()
    }

    #[test]
    fn thumbnail_deterministic_bytes() {
        let src = tiny_png(100, 60);
        let a = render_thumbnail_png(&src, 64).unwrap();
        let b = render_thumbnail_png(&src, 64).unwrap();
        assert_eq!(a, b, "同输入逐字节一致（§29 deterministic）");
        // 尺寸：最长边 64。
        let decoded = image::load_from_memory(&a).unwrap();
        assert_eq!(decoded.width().max(decoded.height()), 64);
    }

    #[test]
    fn jpeg_source_supported() {
        let src = tiny_jpeg(200, 100);
        let png = render_thumbnail_png(&src, 64).unwrap();
        let decoded = image::load_from_memory(&png).unwrap();
        assert_eq!(decoded.width().max(decoded.height()), 64);
    }

    #[test]
    fn no_upscale_beyond_source() {
        let src = tiny_png(32, 32);
        let out = render_thumbnail_png(&src, 256).unwrap();
        let decoded = image::load_from_memory(&out).unwrap();
        assert_eq!(decoded.width(), 32, "小图不放大");
    }

    #[test]
    fn hostile_inputs_rejected_no_panic() {
        // 非图片字节。
        assert!(render_thumbnail_png(b"not an image", 64).is_err());
        // 巨型声明维度的小 PNG（decompression abuse → limits 拒绝）。
        let mut hostile = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        hostile.extend_from_slice(&[0u8; 64]); // 垃圾 IHDR 区
        assert!(render_thumbnail_png(&hostile, 64).is_err());
        // 截断的真实 PNG。
        let real = tiny_png(64, 64);
        assert!(render_thumbnail_png(&real[..real.len() / 2], 64).is_err());
    }

    #[test]
    fn cache_get_create_and_recover_from_corruption() {
        let dir = std::env::temp_dir().join(format!("aura-m3-thumb-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cache = ThumbnailCache::new(&dir);
        let src = tiny_png(100, 100);

        let p1 = cache.get_or_create("hash1", &src, 64).unwrap();
        assert!(p1.is_file());
        assert!(
            cache.get_or_create("hash1", &src, 64).is_ok(),
            "二次命中缓存"
        );

        // 模拟缓存损坏：写垃圾文件 → invalidate → 重建成功。
        std::fs::write(cache.path_for("hash1", 64), b"corrupt").unwrap();
        cache.invalidate("hash1").unwrap();
        let p2 = cache.get_or_create("hash1", &src, 64).unwrap();
        let bytes = std::fs::read(p2).unwrap();
        assert_eq!(
            bytes,
            render_thumbnail_png(&src, 64).unwrap(),
            "重建结果一致"
        );

        // 无 .tmp 残留（§29 atomic write）。
        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|x| x == "tmp"))
            .collect();
        assert!(leftovers.is_empty(), "无 tmp 残留: {leftovers:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sniff_helper() {
        assert!(is_probable_image(&tiny_png(2, 2)));
        assert!(is_probable_image(&tiny_jpeg(2, 2)));
        assert!(!is_probable_image(b"xx"));
    }
}
