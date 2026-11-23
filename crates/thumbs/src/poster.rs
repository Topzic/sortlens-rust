//! Video poster frames: a clip's first sync sample, decoded like a photo's preview.
//!
//! A video cell in the grid is a real image, not a "VID" glyph: the first sync sample of the first
//! video track is decoded through `craft-video` and converted to RGBA honouring the stream's own
//! colour tags — matrix, transfer, primaries and range — so a wide-gamut phone clip keeps its
//! colour. The demuxer reads straight from disk (`craft_media::FileReader`), because a clip can be
//! gigabytes and a poster needs none of the file after its first GOP.
//!
//! One frame is all a poster needs, so the decode loop feeds the sync sample and at most a few
//! samples after it — enough for the decoder's reorder buffer to release the first picture — and
//! flushes if none came. The result goes through the ordinary [`crate::decode_file`] route, so the
//! same `Service`, cache and failure tracking that serve photos serve clips.

#![forbid(unsafe_code)]

use crate::{Thumbnail, resize};
use craft_color::{Matrix, Range, Transfer, linear_to_srgb_u8, normalize_c, normalize_y, to_linear, ycbcr_to_rgb};
use craft_media::isobmff::{ByteSource, Mp4File, TrackKind};
use craft_video::frame::{ColorFamily, VideoFrame};
use sortlens_core::{Error, Result};
use std::path::Path;

/// How many samples past the first sync one may be fed before giving up on a picture.
///
/// The first sync sample is usually an IDR and the decoder holds it in the reorder buffer until a
/// later picture or a flush releases it, so in practice the loop feeds two and gets one out. The
/// bound is what keeps a stream with a broken GOP from decoding half a clip for one cell.
const FEED_LIMIT: usize = 8;

/// Whether a file's *header* names an ISO base-media video track.
///
/// By header rather than by name, so a clip renamed `.jpg` still gets its poster and a still named
/// `.mp4` does not. Raw containers (a CR3's brand) never reach here: [`crate::decode_file`] routes
/// raws by name before anything is read.
pub(super) fn header_is_video(path: &Path) -> bool {
    sniff_header(path) == Some(sortlens_core::media::Magic::Video)
}

/// The format magic of a file's first bytes, if it has enough of them.
fn sniff_header(path: &Path) -> Option<sortlens_core::media::Magic> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).ok()?;
    let mut head = [0u8; 64];
    let read = file.read(&mut head).ok()?;
    Some(sortlens_core::media::sniff(&head[..read]))
}

/// Decode the first sync sample of a video into an RGBA poster no larger than `max_edge`.
pub fn decode_video_poster(path: &Path, max_edge: u32) -> Result<Thumbnail> {
    let reader = craft_media::FileReader::open(path).map_err(|error| Error::io(path.display().to_string(), error))?;
    let container = craft_media::isobmff::open(&reader).map_err(|error| Error::Invalid(format!("video: {error}")))?;
    let (track_index, track) = container
        .tracks
        .iter()
        .enumerate()
        .find(|(_, track)| track.kind == TrackKind::Video)
        .ok_or_else(|| Error::UnsupportedFormat("video file has no video track".into()))?;

    let params = track.codec_params();
    let extradata = params.extradata.clone();
    let mut decoder = match params.codec {
        craft_media::CodecId::H264 => {
            let mut decoder = craft_video::h264::decode::Decoder::new();
            decoder
                .configure_avcc(&extradata)
                .map_err(|error| Error::Invalid(format!("video: h264 setup: {error}")))?;
            PosterDecoder::H264(Box::new(decoder))
        }
        craft_media::CodecId::Hevc => {
            let mut decoder = craft_video::hevc::decode::Decoder::new();
            decoder
                .configure_hvcc(&extradata)
                .map_err(|error| Error::Invalid(format!("video: hevc setup: {error}")))?;
            PosterDecoder::Hevc(Box::new(decoder))
        }
        other => {
            return Err(Error::UnsupportedFormat(format!(
                "video codec {} has no poster decoder",
                other.name()
            )));
        }
    };

    // The first sync sample, wherever the container says it is. A track that marks none falls back
    // to the first sample: a single-picture clip is legal and a poster of a broken stream is still
    // worth attempting rather than refusing the cell outright.
    let first_sync = track
        .sync_samples()
        .first()
        .map(|&index| index as usize)
        .or_else(|| (!track.samples.is_empty()).then_some(0))
        .ok_or_else(|| Error::UnsupportedFormat("video track has no samples".into()))?;

    let frame = decode_first_frame(&reader, &container, track_index, first_sync, &mut decoder)?;
    let rgba = frame_to_rgba(&frame)?;
    let thumbnail = resize(&rgba, frame.width(), frame.height(), 4, max_edge);
    // A phone held upright records landscape and says so in the `tkhd` matrix; the catalog's width
    // and height already describe the *displayed* frame, so the poster turns to match them.
    Ok(rotate(thumbnail, track.display_rotation().unwrap_or(0)))
}

/// The two codecs a poster supports, behind one call site.
enum PosterDecoder {
    H264(Box<craft_video::h264::decode::Decoder>),
    Hevc(Box<craft_video::hevc::decode::Decoder>),
}

impl PosterDecoder {
    fn decode(&mut self, data: &[u8], pts: i64) -> Result<Option<VideoFrame>> {
        match self {
            Self::H264(decoder) => {
                let pictures = decoder
                    .decode(data, pts)
                    .map_err(|error| Error::Invalid(format!("video: {error}")))?;
                pictures_into_frames(pictures)
            }
            Self::Hevc(decoder) => {
                let pictures = decoder
                    .decode(data, pts)
                    .map_err(|error| Error::Invalid(format!("video: {error}")))?;
                pictures_into_frames(pictures)
            }
        }
    }

    fn flush(&mut self) -> Result<Option<VideoFrame>> {
        match self {
            Self::H264(decoder) => pictures_into_frames(decoder.flush()),
            Self::Hevc(decoder) => pictures_into_frames(decoder.flush()),
        }
    }
}

/// The first decoded picture that survives the conversion into a [`VideoFrame`].
fn pictures_into_frames<P: IntoFrame>(pictures: Vec<P>) -> Result<Option<VideoFrame>> {
    pictures.into_iter().next().map(IntoFrame::into_frame).transpose()
}

/// A decoder picture that becomes a [`VideoFrame`]: the two decoders a poster supports.
trait IntoFrame {
    fn into_frame(self) -> Result<VideoFrame>;
}

impl IntoFrame for craft_video::h264::decode::Picture {
    fn into_frame(self) -> Result<VideoFrame> {
        VideoFrame::try_from(self).map_err(|error| Error::Invalid(format!("video frame: {error}")))
    }
}

impl IntoFrame for craft_video::hevc::decode::Picture {
    fn into_frame(self) -> Result<VideoFrame> {
        VideoFrame::try_from(self).map_err(|error| Error::Invalid(format!("video frame: {error}")))
    }
}

/// Feed the first sync sample and, at most, a few after it until the decoder releases a picture.
///
/// The sync sample's own bytes are read once and fed first; the loop only reaches the next samples
/// when the decoder's reorder buffer is holding the first picture back (B-frames, or an output
/// ordering that waits for the following access unit). A flush is the last resort: it releases
/// whatever is buffered, so a first picture comes out even from a stream that ends abruptly.
fn decode_first_frame(
    reader: &impl ByteSource,
    container: &Mp4File,
    track: usize,
    first_sync: usize,
    decoder: &mut PosterDecoder,
) -> Result<VideoFrame> {
    let samples = &container.tracks[track].samples;
    let mut fed = 0;
    while fed <= FEED_LIMIT {
        let data = if fed == 0 {
            container
                .read_sample(reader, track, first_sync)
                .map_err(|error| Error::Invalid(format!("video: {error}")))?
        } else {
            let index = first_sync + fed;
            match container.read_sample(reader, track, index) {
                Ok(data) => data,
                Err(_) => break,
            }
        };
        let pts = samples.get(first_sync + fed).map_or(0, |sample| sample.pts);
        if let Some(frame) = decoder.decode(&data, pts)? {
            return Ok(frame);
        }
        fed += 1;
    }
    decoder
        .flush()?
        .ok_or_else(|| Error::UnsupportedFormat("video track has no decodable picture".into()))
}

/// Convert a decoded frame to sRGB-encoded RGBA8, honouring the stream's own colour tags.
///
/// The arithmetic lives in `craft-color`; this function is the plumbing: range and matrix decide
/// what the samples *mean*, transfer and primaries decide how they are *encoded and gamut-mapped*
/// for display. A grey pixel stays grey through any of it — the one invariant a test can hold.
fn frame_to_rgba(frame: &VideoFrame) -> Result<Vec<u8>> {
    if frame.family() != ColorFamily::YCbCr {
        return Err(Error::UnsupportedFormat(format!(
            "video frame in the {:?} colour family has no poster conversion",
            frame.family()
        )));
    }
    let width = frame.width() as usize;
    let height = frame.height() as usize;
    let subsampling = frame.subsampling();
    let (sx, sy) = (subsampling.x, subsampling.y);
    let planes = frame.planes();
    let (y_plane, u_plane, v_plane) = match planes {
        [y, u, v] => (y, u, v),
        _ => return Err(Error::UnsupportedFormat("video frame does not have three planes".into())),
    };
    let tags = frame.color;
    // An unspecified matrix (code 2) reads as BT.709 for HD and BT.601 for SD elsewhere in the
    // suite (`craft_frame::default_matrix`); a thumbnail of an unlabelled clip follows the same
    // rule so a cell and a compositor frame cannot disagree.
    let matrix = Matrix::from_code(tags.matrix)
        .or_else(|| (width <= 720 && height <= 576).then_some(Matrix::Bt601))
        .unwrap_or(Matrix::Bt709);
    let range = if tags.full_range { Range::Full } else { Range::Limited };
    let transfer = Transfer::from_code(tags.transfer).unwrap_or_default();

    let mut rgba = Vec::with_capacity(width * height * 4);
    // A sample as (value, bits): the two normalisers need both, and a plane may be 8- or 16-bit.
    let raw = |plane: &craft_video::frame::Plane, y: u32, x: u32| -> Result<(u32, u32)> {
        let bits = u32::from(plane.bit_depth());
        let value = match plane.samples() {
            craft_video::frame::Samples::U8(_) => u32::from(
                *plane
                    .row_u8(y)
                    .and_then(|row| row.get(x as usize))
                    .ok_or_else(|| Error::Invalid("video frame plane is smaller than its size".into()))?,
            ),
            craft_video::frame::Samples::U16(_) => u32::from(
                *plane
                    .row_u16(y)
                    .and_then(|row| row.get(x as usize))
                    .ok_or_else(|| Error::Invalid("video frame plane is smaller than its size".into()))?,
            ),
        };
        Ok((value, bits))
    };
    let luma = |plane: &craft_video::frame::Plane, y: u32, x: u32| -> Result<f32> {
        let (value, bits) = raw(plane, y, x)?;
        normalize_y(value, bits, range).map_err(|error| Error::Invalid(format!("video frame: {error}")))
    };
    let chroma = |plane: &craft_video::frame::Plane, y: u32, x: u32| -> Result<f32> {
        // Chroma is centred: Cb and Cr read as -0.5..0.5 around neutral 128.
        let (value, bits) = raw(plane, y, x)?;
        normalize_c(value, bits, range).map_err(|error| Error::Invalid(format!("video frame: {error}")))
    };
    for y in 0..height as u32 {
        let cy = y >> sy;
        for x in 0..width as u32 {
            let cx = x >> sx;
            let yy = luma(y_plane, y, x)?;
            let cb = chroma(u_plane, cy, cx)?;
            let cr = chroma(v_plane, cy, cx)?;
            let [r, g, b] = ycbcr_to_rgb(yy, cb, cr, matrix);
            let [r, g, b] = gamut_to_srgb([r, g, b], tags.primaries);
            rgba.extend_from_slice(&[
                linear_to_srgb_u8(to_linear(r.clamp(0.0, 1.0), transfer)),
                linear_to_srgb_u8(to_linear(g.clamp(0.0, 1.0), transfer)),
                linear_to_srgb_u8(to_linear(b.clamp(0.0, 1.0), transfer)),
                255,
            ]);
        }
    }
    Ok(rgba)
}

/// Gamut-map linear RGB from the stream's primaries into sRGB/BT.709 primaries.
///
/// Phone clips are BT.709 or P3D65; BT.2020 arrives with HDR. Same white point (D65) in every case
/// here, so the mapping is one 3×3 matrix with no chromatic adaptation. Anything unlabelled, or a
/// BT.601 variant close enough that a thumbnail cannot tell, passes through unchanged.
fn gamut_to_srgb(lin: [f32; 3], primaries: u8) -> [f32; 3] {
    let (to_xyz, from_xyz): ([f32; 9], [f32; 9]) = match primaries {
        // BT.2020 (9) and P3D65 (12); the rest already name the display's primaries closely enough.
        9 => (
            [
                0.636_958,
                0.144_616_9,
                0.168_881,
                0.262_700_2,
                0.677_998_1,
                0.059_301_7,
                0.0,
                0.028_072_7,
                1.060_985,
            ],
            [
                3.240_454_2,
                -1.537_138_5,
                -0.498_531_4,
                -0.969_266,
                1.876_010_8,
                0.041_556,
                0.055_643_4,
                -0.204_025_9,
                1.057_225_2,
            ],
        ),
        12 => (
            [
                0.486_570_9,
                0.265_667_7,
                0.198_217_3,
                0.228_974_6,
                0.691_738_5,
                0.079_286_9,
                0.0,
                0.045_113_4,
                1.043_944_4,
            ],
            [
                3.240_454_2,
                -1.537_138_5,
                -0.498_531_4,
                -0.969_266,
                1.876_010_8,
                0.041_556,
                0.055_643_4,
                -0.204_025_9,
                1.057_225_2,
            ],
        ),
        _ => return lin,
    };
    let xyz = [
        to_xyz[0] * lin[0] + to_xyz[1] * lin[1] + to_xyz[2] * lin[2],
        to_xyz[3] * lin[0] + to_xyz[4] * lin[1] + to_xyz[5] * lin[2],
        to_xyz[6] * lin[0] + to_xyz[7] * lin[1] + to_xyz[8] * lin[2],
    ];
    [
        from_xyz[0] * xyz[0] + from_xyz[1] * xyz[1] + from_xyz[2] * xyz[2],
        from_xyz[3] * xyz[0] + from_xyz[4] * xyz[1] + from_xyz[5] * xyz[2],
        from_xyz[6] * xyz[0] + from_xyz[7] * xyz[1] + from_xyz[8] * xyz[2],
    ]
}

/// Quarter-turn the thumbnail clockwise.
///
/// Phone clips record sideways and say so in the container's `tkhd` matrix; `track.display_rotation`
/// reports the quarter turns a viewer must apply, so the poster applies them too and a portrait
/// clip shows portrait. Non-rotational matrices (mirrors, scales) report `None` and no turn.
fn rotate(thumbnail: Thumbnail, quarter_turns: u8) -> Thumbnail {
    let Thumbnail { width, height, rgba } = thumbnail;
    let turn = quarter_turns % 4;
    if turn == 0 {
        return Thumbnail { width, height, rgba };
    }
    let (out_w, out_h) = match turn {
        1 | 3 => (height, width),
        _ => (width, height),
    };
    let mut out = vec![0u8; out_w as usize * out_h as usize * 4];
    for y in 0..height {
        for x in 0..width {
            let (x2, y2) = match turn {
                1 => (height - 1 - y, x),
                2 => (width - 1 - x, height - 1 - y),
                _ => (y, width - 1 - x),
            };
            let source = (y as usize * width as usize + x as usize) * 4;
            let target = (y2 as usize * out_w as usize + x2 as usize) * 4;
            out[target..target + 4].copy_from_slice(&rgba[source..source + 4]);
        }
    }
    Thumbnail {
        width: out_w,
        height: out_h,
        rgba: out,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    /// A luma ramp across the frame, neutral chroma: whatever matrix the stream declares, a
    /// chroma-neutral pixel is grey, so the decode can be judged on luma alone.
    fn ramp_frame(width: u32, height: u32) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let luma: Vec<u8> = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x * 255 / width.max(1)).max(y % 2) as u8))
            .collect();
        let chroma = vec![128u8; width as usize * height as usize / 4];
        (luma, chroma.clone(), chroma)
    }

    /// Encode `frames` H.264 pictures, length-prefixed, ready for an MP4 track. The first frame
    /// carries the ramp; every later one is uniform mid grey, so a poster that shows a later frame
    /// — a reorder-buffer bug — reads as a flat image and fails the assertions that look for the ramp.
    fn encode_h264(width: u32, height: u32, frames: usize) -> (craft_media::isobmff::AvcConfig, Vec<Vec<u8>>) {
        use craft_video::h264::{Encoder, EncoderConfig, PacketFormat, Profile, YuvFrame};
        let mut config = EncoderConfig::new(width, height, 24, 1);
        config.profile = Profile::Baseline;
        config.bframes = 0;
        config.format = PacketFormat::LengthPrefixed;
        config.keyint = 8;
        config.threads = 1;
        config.writer = "sortlens-thumbs-test".into();
        let mut encoder = Encoder::new(config).expect("encoder");
        let (ramp_y, ramp_u, ramp_v) = ramp_frame(width, height);
        let flat_y = vec![60u8; width as usize * height as usize];
        let flat_u = vec![128u8; width as usize * height as usize / 4];
        let mut samples = Vec::new();
        for pts in 0..i64::try_from(frames).unwrap_or(0) {
            let (y, u, v) = if pts == 0 {
                (&ramp_y, &ramp_u, &ramp_v)
            } else {
                (&flat_y, &flat_u, &flat_u)
            };
            let frame = YuvFrame {
                y,
                u,
                v,
                y_stride: width as usize,
                uv_stride: width as usize / 2,
            };
            for packet in encoder.encode(&frame, pts).expect("encode") {
                samples.push(packet.data);
            }
        }
        samples.extend(encoder.flush().into_iter().map(|packet| packet.data));
        let (sps, pps) = encoder.sps_pps();
        (craft_media::isobmff::AvcConfig::new(vec![sps], vec![pps], 4), samples)
    }

    /// Encode two HEVC pictures, length-prefixed, ready for an MP4 track.
    fn encode_hevc(width: u32, height: u32) -> (craft_media::isobmff::HevcConfig, Vec<Vec<u8>>) {
        use craft_video::hevc::{Encoder, EncoderConfig, Frame};
        let mut encoder = Encoder::new(EncoderConfig::new(width, height, 24, 1)).expect("encoder");
        let (y, u, _v) = ramp_frame(width, height);
        let y16: Vec<u16> = y.iter().map(|&sample| u16::from(sample)).collect();
        let u16_planes: Vec<u16> = u.iter().map(|&sample| u16::from(sample)).collect();
        let frame = Frame {
            y: &y16,
            u: &u16_planes,
            v: &u16_planes,
            y_stride: width as usize,
            uv_stride: width as usize / 2,
        };
        let mut samples = Vec::new();
        let first = encoder.encode(&frame).expect("encode");
        samples.push(first.data);
        if let Ok(second) = encoder.encode(&frame) {
            samples.push(second.data);
        }
        let hvcc = encoder.hvcc();
        (craft_media::isobmff::HevcConfig::parse(&hvcc).expect("hvcC"), samples)
    }

    /// Mux encoded samples into an MP4 file on disk, first sample marked sync.
    fn mux(dir: &Path, name: &str, entry: craft_media::isobmff::SampleEntry, timescale: u32, samples: &[Vec<u8>]) -> std::path::PathBuf {
        use craft_media::isobmff::{Mp4Writer, WriteSample, WriterOptions};
        let sink = std::io::Cursor::new(Vec::new());
        let mut writer = Mp4Writer::new(sink, WriterOptions::new(craft_media::isobmff::Brand::Mp4)).expect("writer");
        let track = writer
            .add_track(craft_media::isobmff::TrackConfig::new(entry, timescale))
            .expect("track");
        for (index, data) in samples.iter().enumerate() {
            writer
                .write_sample(
                    track,
                    WriteSample {
                        data,
                        duration: 10,
                        composition_offset: 0,
                        is_sync: index == 0,
                    },
                )
                .expect("sample");
        }
        let bytes = writer.finish().expect("finish").into_inner();
        let path = dir.join(name);
        std::fs::write(&path, &bytes).expect("write");
        path
    }

    /// A fixture directory that cleans up after itself.
    struct Dir(std::path::PathBuf);
    impl Dir {
        fn new(tag: &str) -> Self {
            let path = std::env::temp_dir().join(format!("sortlens-poster-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("dir");
            Self(path)
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// The poster of a muxed H.264 clip is its first frame: a left-dark, right-light grey ramp that
    /// survived encode, mux, demux, decode and the colour conversion.
    #[test]
    fn a_muxed_h264_clip_yields_its_first_frame_as_a_poster() {
        let dir = Dir::new("h264");
        let (avc, samples) = encode_h264(64, 48, 3);
        let entry = craft_media::isobmff::SampleEntry::video(
            craft_media::isobmff::FourCc(*b"avc1"),
            craft_media::isobmff::CodecConfig::Avc(avc),
            64,
            48,
        );
        let path = mux(&dir.0, "clip.mp4", entry, 300, &samples);

        assert!(header_is_video(&path), "the header says video");

        // Through the ordinary dispatcher, the way the thumbnail service reaches it: no special case.
        let poster = crate::decode_file(&path, 128).expect("poster");
        assert_eq!((poster.width, poster.height), (64, 48), "no upscale and no rotation here");
        assert!(poster.is_consistent());
        let pixel = |x: u32| {
            let index = ((poster.height as usize / 2) * poster.width as usize + x as usize) * 4;
            [poster.rgba[index], poster.rgba[index + 1], poster.rgba[index + 2]]
        };
        let left = pixel(1);
        let right = pixel(poster.width - 2);
        assert!(right[0] > left[0] + 80, "the ramp runs left to right: {left:?} then {right:?}");
        for rgb in [left, right] {
            assert!(
                rgb[0].abs_diff(rgb[1]) <= 4 && rgb[1].abs_diff(rgb[2]) <= 4,
                "neutral chroma stays grey through the colour conversion: {rgb:?}"
            );
        }
        // The poster is the *first* frame's ramp, not a later uniform-grey frame.
        assert!(right[0] > 200, "a 64-wide ramp at its right end is bright: {right:?}");

        // And it lands in the content-addressed cache like any preview: a second decode is a hit.
        let cache = crate::Cache::new(dir.0.join("cache"));
        cache.put(7, 128, &poster).expect("put");
        assert_eq!(cache.get(7, 128), Some(poster));
    }

    /// The same through the HEVC path: a Main-profile clip muxed with an `hvcC` record.
    #[test]
    fn a_muxed_hevc_clip_yields_its_first_frame_as_a_poster() {
        let dir = Dir::new("hevc");
        let (hvcc, samples) = encode_hevc(64, 48);
        let entry = craft_media::isobmff::SampleEntry::video(
            craft_media::isobmff::FourCc(*b"hvc1"),
            craft_media::isobmff::CodecConfig::Hevc(hvcc),
            64,
            48,
        );
        let path = mux(&dir.0, "clip.mp4", entry, 300, &samples);
        let poster = crate::decode_file(&path, 128).expect("poster");
        assert_eq!((poster.width, poster.height), (64, 48));
        let index = ((poster.height as usize / 2) * poster.width as usize + poster.width as usize - 2) * 4;
        assert!(poster.rgba[index] > 200, "the right end of the ramp is bright");
    }

    /// A clip renamed `.jpg` still gets its poster and a still named `.mp4` is still a still:
    /// the route is the header, not the extension.
    #[test]
    fn the_route_is_the_header_not_the_name() {
        let dir = Dir::new("route");
        let (avc, samples) = encode_h264(32, 32, 1);
        let entry = craft_media::isobmff::SampleEntry::video(
            craft_media::isobmff::FourCc(*b"avc1"),
            craft_media::isobmff::CodecConfig::Avc(avc),
            32,
            32,
        );
        let clip = mux(&dir.0, "clip.jpg", entry, 300, &samples);
        assert!(header_is_video(&clip));
        let poster = crate::decode_file(&clip, 128).expect("poster by header");
        assert_eq!(poster.width, 32);

        let jpeg =
            craft_jpeg::encode_baseline(&vec![90u8; 32 * 32 * 3], 32, 32, 3, 80, craft_jpeg::ChromaSubsampling::S444, &[]).expect("encode");
        let still = dir.0.join("still.mp4");
        std::fs::write(&still, &jpeg).expect("write");
        assert!(!header_is_video(&still));
        let decoded = crate::decode_file(&still, 128).expect("still by header");
        assert_eq!(decoded.width, 32);
    }

    /// A file that is not a video at all, and a truncated container, are refused with a reason
    /// rather than a panic: the cell falls back to its glyph.
    #[test]
    fn garbage_and_truncation_are_refused_with_a_reason() {
        let dir = Dir::new("broken");
        let (avc, samples) = encode_h264(32, 32, 1);
        let entry = craft_media::isobmff::SampleEntry::video(
            craft_media::isobmff::FourCc(*b"avc1"),
            craft_media::isobmff::CodecConfig::Avc(avc),
            32,
            32,
        );
        let path = mux(&dir.0, "clip.mp4", entry, 300, &samples);

        let bytes = std::fs::read(&path).expect("read");
        let truncated = dir.0.join("cut.mp4");
        std::fs::write(&truncated, &bytes[..bytes.len() / 2]).expect("write");
        assert!(header_is_video(&truncated), "half a container still names a video");
        let error = decode_video_poster(&truncated, 128).expect_err("truncated");
        assert!(error.to_string().contains("video"), "the reason says where it came from: {error}");

        let none = dir.0.join("none.bin");
        std::fs::write(&none, b"not a container").expect("write");
        assert!(!header_is_video(&none));
        let error = decode_video_poster(&none, 128).expect_err("not a video");
        assert!(matches!(error, sortlens_core::Error::Invalid(_)), "{error}");
    }

    /// The first frame is what comes out even though the decoder cannot release it until a later
    /// access unit has been decoded: the later frames are flat grey, so a poster of anything but
    /// frame one is a flat image and the assertion says so.
    #[test]
    fn the_reorder_buffer_cannot_make_a_poster_of_the_wrong_frame() {
        let dir = Dir::new("reorder");
        let (avc, samples) = encode_h264(64, 48, 3);
        let entry = craft_media::isobmff::SampleEntry::video(
            craft_media::isobmff::FourCc(*b"avc1"),
            craft_media::isobmff::CodecConfig::Avc(avc),
            64,
            48,
        );
        let path = mux(&dir.0, "clip.mp4", entry, 300, &samples);
        let poster = decode_video_poster(&path, 128).expect("poster");
        let index = ((poster.height as usize / 2) * poster.width as usize + poster.width as usize - 2) * 4;
        assert!(poster.rgba[index] > 200, "the released frame still has the ramp");
    }

    /// Rotation: a quarter-turn moves the top-left corner where a viewer would put it, and the
    /// dimensions swap — the shape a portrait clip's cell already has from `tkhd`.
    #[test]
    fn quarter_turns_rotate_the_poster() {
        let two_by_one = Thumbnail {
            width: 2,
            height: 1,
            rgba: vec![10, 11, 12, 255, 20, 21, 22, 255],
        };
        let turned = rotate(two_by_one.clone(), 1);
        assert_eq!((turned.width, turned.height), (1, 2));
        assert_eq!(
            turned.rgba,
            vec![10, 11, 12, 255, 20, 21, 22, 255],
            "the left pixel becomes the top row"
        );

        let half = rotate(two_by_one.clone(), 2);
        assert_eq!(half.rgba, vec![20, 21, 22, 255, 10, 11, 12, 255]);

        let back = rotate(two_by_one.clone(), 3);
        assert_eq!((back.width, back.height), (1, 2));
        assert_eq!(back.rgba, vec![20, 21, 22, 255, 10, 11, 12, 255]);

        assert_eq!(rotate(two_by_one.clone(), 0), rotate(two_by_one.clone(), 4), "four turns is none");
    }
}
