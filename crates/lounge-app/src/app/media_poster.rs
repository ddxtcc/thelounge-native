use image::{ImageBuffer, Rgba};
use std::hash::{Hash, Hasher};
use yuv::{YuvBiPlanarImage, YuvConversionMode, YuvRange, YuvStandardMatrix, yuv_nv12_to_bgra};

use super::*;

#[derive(Clone)]
pub(super) struct Poster {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
}

pub(super) fn generate_poster(url: &str) -> Option<Poster> {
    let parsed = url::Url::parse(url).ok()?;
    let video = gpui_video_player::Video::new(&parsed).ok()?;
    video.set_muted(true);
    let started = std::time::Instant::now();
    let (yuv, width, height) = loop {
        if let Some(frame) = video.current_frame_data()
            && frame.1 > 0
            && frame.2 > 0
        {
            break frame;
        }
        if started.elapsed() > std::time::Duration::from_secs(3) {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
    let width_usize = width as usize;
    let height_usize = height as usize;
    let y_size = width_usize * height_usize;
    let uv_size = y_size / 2;
    if yuv.len() < y_size + uv_size {
        return None;
    }
    let image = YuvBiPlanarImage {
        y_plane: &yuv[..y_size],
        y_stride: width,
        uv_plane: &yuv[y_size..y_size + uv_size],
        uv_stride: width,
        width,
        height,
    };
    let mut bgra = vec![0u8; width_usize * height_usize * 4];
    let mut converted = false;
    for (range, matrix) in [
        (YuvRange::Full, YuvStandardMatrix::Bt709),
        (YuvRange::Limited, YuvStandardMatrix::Bt709),
        (YuvRange::Limited, YuvStandardMatrix::Bt601),
    ] {
        if yuv_nv12_to_bgra(
            &image,
            &mut bgra,
            width * 4,
            range,
            matrix,
            YuvConversionMode::Balanced,
        )
        .is_ok()
        {
            converted = true;
            break;
        }
    }
    if !converted {
        return None;
    }
    let mut rgba = vec![0u8; bgra.len()];
    for (source, target) in bgra
        .as_chunks::<4>()
        .0
        .iter()
        .zip(rgba.as_chunks_mut::<4>().0)
    {
        target[0] = source[2];
        target[1] = source[1];
        target[2] = source[0];
        target[3] = source[3];
    }
    let buffer: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(width, height, rgba)?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    let path = std::env::temp_dir().join(format!("lounge-native-poster-{:x}.png", hasher.finish()));
    buffer.save(&path).ok()?;
    Some(Poster {
        path,
        width,
        height,
    })
}
