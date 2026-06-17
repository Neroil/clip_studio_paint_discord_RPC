use std::{fs, io::Cursor, mem::zeroed, path::Path};

use crate::app_state::Settings;
use reqwest::blocking::{multipart, Client};
use reqwest::header::USER_AGENT;
use tauri::{AppHandle, Manager};

const OBS_PNG_LUT_SIZE: usize = 64;
const OBS_PNG_LUT_TILES_PER_AXIS: usize = 8;
const OBS_PNG_LUT_DIMENSION: u32 = (OBS_PNG_LUT_SIZE * OBS_PNG_LUT_TILES_PER_AXIS) as u32;

pub struct ShareResult {
    pub url: String,
}

pub fn capture_and_upload(
    app: &AppHandle,
    settings: &Settings,
) -> Result<ShareResult, CaptureShareError> {
    let mut png = capture_clip_studio_png()?;
    if settings.apply_screenshot_lut {
        png = apply_screenshot_lut(&png, &settings.screenshot_lut_path)?;
    }

    let saved_path = app
        .path()
        .app_cache_dir()
        .map_err(|_| CaptureShareError::AppCacheDir)?
        .join("latest-clip-studio-capture.png");

    if let Some(parent) = saved_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&saved_path, &png)?;

    let url = upload_screenshot(png)?;
    Ok(ShareResult { url })
}

fn upload_screenshot(png: Vec<u8>) -> Result<String, CaptureShareError> {
    upload_to_uguu(png)
}

fn apply_screenshot_lut(png: &[u8], lut_path: &str) -> Result<Vec<u8>, CaptureShareError> {
    let lut_path = lut_path.trim();
    if lut_path.is_empty() {
        return Err(CaptureShareError::LutPathMissing);
    }

    let lut = PngLut::load(Path::new(lut_path))?;
    let (width, height, mut rgba) = decode_png_rgba(png, DecodeTarget::Screenshot)?;
    apply_png_lut(&mut rgba, &lut);
    encode_png(width, height, &rgba)
}

fn upload_to_uguu(png: Vec<u8>) -> Result<String, CaptureShareError> {
    let part = multipart::Part::bytes(png)
        .file_name("clip-studio-presence.png")
        .mime_str("image/png")?;
    let form = multipart::Form::new().part("files[]", part);

    let client = Client::builder()
        .user_agent("ClipStudioPresence/0.1")
        .http1_only()
        .build()?;

    let response = client
        .post("https://uguu.se/upload?output=text")
        .multipart(form)
        .header(USER_AGENT, "ClipStudioPresence/0.1")
        .send()
        .map_err(|source| CaptureShareError::UploadRequest {
            message: upload_request_message("Uguu", &source),
        })?;

    let status = response.status();
    let body = response
        .text()
        .map_err(|source| CaptureShareError::UploadResponseBody {
            status,
            message: upload_response_body_message("Uguu", status, &source),
        })?;

    if status.is_success() {
        let url = body.trim().to_string();
        if url.starts_with("https://") || url.starts_with("http://") {
            Ok(url)
        } else {
            Err(CaptureShareError::UploadRejected(format!(
                "Uguu replied with HTTP {status}, but the body was not a direct URL. Body text: {url}"
            )))
        }
    } else {
        Err(CaptureShareError::UploadFailed { status, body })
    }
}

#[derive(Clone, Copy)]
enum DecodeTarget<'a> {
    Screenshot,
    Lut(&'a Path),
}

impl DecodeTarget<'_> {
    fn decode_error(self, source: png::DecodingError) -> CaptureShareError {
        match self {
            Self::Screenshot => CaptureShareError::ScreenshotDecode(source),
            Self::Lut(path) => CaptureShareError::LutDecode {
                path: path.display().to_string(),
                source,
            },
        }
    }

    fn indexed_error(self) -> CaptureShareError {
        match self {
            Self::Screenshot => CaptureShareError::ScreenshotDecodeMessage(
                "Indexed-color screenshots are not supported for LUT processing".to_string(),
            ),
            Self::Lut(path) => CaptureShareError::LutParse {
                path: path.display().to_string(),
                message:
                    "Indexed-color PNG LUTs are not supported; use an RGB or RGBA OBS-style LUT PNG"
                        .to_string(),
            },
        }
    }
}

fn decode_png_rgba(
    png_bytes: &[u8],
    target: DecodeTarget<'_>,
) -> Result<(u32, u32, Vec<u8>), CaptureShareError> {
    let mut decoder = png::Decoder::new(Cursor::new(png_bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|source| target.decode_error(source))?;
    let buffer_len = reader
        .output_buffer_size()
        .unwrap_or((reader.info().width as usize) * (reader.info().height as usize) * 4);
    let mut buffer = vec![0; buffer_len];
    let info = reader
        .next_frame(&mut buffer)
        .map_err(|source| target.decode_error(source))?;
    let bytes = &buffer[..info.buffer_size()];

    match info.color_type {
        png::ColorType::Rgba => Ok((info.width, info.height, bytes.to_vec())),
        png::ColorType::Rgb => Ok((info.width, info.height, rgb_bytes_to_rgba(bytes))),
        png::ColorType::Grayscale => Ok((info.width, info.height, grayscale_bytes_to_rgba(bytes))),
        png::ColorType::GrayscaleAlpha => Ok((
            info.width,
            info.height,
            grayscale_alpha_bytes_to_rgba(bytes),
        )),
        png::ColorType::Indexed => Err(target.indexed_error()),
    }
}

fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, CaptureShareError> {
    let mut output = Cursor::new(Vec::new());
    let mut encoder = png::Encoder::new(&mut output, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(rgba)?;
    drop(writer);
    Ok(output.into_inner())
}

#[derive(Clone, Debug)]
struct PngLut {
    size: usize,
    pixels: Vec<u8>,
}

impl PngLut {
    fn load(path: &Path) -> Result<Self, CaptureShareError> {
        let data = fs::read(path).map_err(|source| CaptureShareError::LutRead {
            path: path.display().to_string(),
            source,
        })?;

        let (width, height, pixels) = decode_png_rgba(&data, DecodeTarget::Lut(path))?;
        if width != OBS_PNG_LUT_DIMENSION || height != OBS_PNG_LUT_DIMENSION {
            return Err(CaptureShareError::LutParse {
                path: path.display().to_string(),
                message: format!(
                    "OBS-style PNG LUTs must be {OBS_PNG_LUT_DIMENSION}x{OBS_PNG_LUT_DIMENSION}, but this file is {width}x{height}"
                ),
            });
        }

        Ok(Self {
            size: OBS_PNG_LUT_SIZE,
            pixels,
        })
    }

    fn sample(&self, r: f32, g: f32, b: f32) -> [f32; 3] {
        let max_index = (self.size - 1) as f32;
        let rx = (r.clamp(0.0, 1.0)) * max_index;
        let gx = (g.clamp(0.0, 1.0)) * max_index;
        let bx = (b.clamp(0.0, 1.0)) * max_index;

        let r0 = rx.floor() as usize;
        let g0 = gx.floor() as usize;
        let b0 = bx.floor() as usize;
        let r1 = (r0 + 1).min(self.size - 1);
        let g1 = (g0 + 1).min(self.size - 1);
        let b1 = (b0 + 1).min(self.size - 1);

        let rt = rx - r0 as f32;
        let gt = gx - g0 as f32;
        let bt = bx - b0 as f32;

        let c000 = self.sample_point(r0, g0, b0);
        let c100 = self.sample_point(r1, g0, b0);
        let c010 = self.sample_point(r0, g1, b0);
        let c110 = self.sample_point(r1, g1, b0);
        let c001 = self.sample_point(r0, g0, b1);
        let c101 = self.sample_point(r1, g0, b1);
        let c011 = self.sample_point(r0, g1, b1);
        let c111 = self.sample_point(r1, g1, b1);

        let c00 = lerp_triplet(c000, c100, rt);
        let c10 = lerp_triplet(c010, c110, rt);
        let c01 = lerp_triplet(c001, c101, rt);
        let c11 = lerp_triplet(c011, c111, rt);
        let c0 = lerp_triplet(c00, c10, gt);
        let c1 = lerp_triplet(c01, c11, gt);
        lerp_triplet(c0, c1, bt)
    }

    fn sample_point(&self, r: usize, g: usize, b: usize) -> [f32; 3] {
        let x = (b % OBS_PNG_LUT_TILES_PER_AXIS) * self.size + r;
        let y = (b / OBS_PNG_LUT_TILES_PER_AXIS) * self.size + g;
        let index = (y * OBS_PNG_LUT_DIMENSION as usize + x) * 4;

        [
            self.pixels[index] as f32 / 255.0,
            self.pixels[index + 1] as f32 / 255.0,
            self.pixels[index + 2] as f32 / 255.0,
        ]
    }
}

fn apply_png_lut(rgba: &mut [u8], lut: &PngLut) {
    for pixel in rgba.chunks_exact_mut(4) {
        let output = lut.sample(
            pixel[0] as f32 / 255.0,
            pixel[1] as f32 / 255.0,
            pixel[2] as f32 / 255.0,
        );

        pixel[0] = (output[0].clamp(0.0, 1.0) * 255.0).round() as u8;
        pixel[1] = (output[1].clamp(0.0, 1.0) * 255.0).round() as u8;
        pixel[2] = (output[2].clamp(0.0, 1.0) * 255.0).round() as u8;
    }
}

fn lerp_triplet(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn rgb_bytes_to_rgba(rgb: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(rgb.len() / 3 * 4);
    for chunk in rgb.chunks_exact(3) {
        rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
    }
    rgba
}

fn grayscale_bytes_to_rgba(gray: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(gray.len() * 4);
    for &value in gray {
        rgba.extend_from_slice(&[value, value, value, 255]);
    }
    rgba
}

fn grayscale_alpha_bytes_to_rgba(gray_alpha: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(gray_alpha.len() / 2 * 4);
    for chunk in gray_alpha.chunks_exact(2) {
        rgba.extend_from_slice(&[chunk[0], chunk[0], chunk[0], chunk[1]]);
    }
    rgba
}

fn upload_request_message(host: &str, error: &reqwest::Error) -> String {
    if error.is_timeout() {
        return format!(
            "The request timed out before {host} answered. The network may be slow or the host may be blocked."
        );
    }

    if error.is_connect() {
        return format!(
            "The app could not connect to {host}. This is usually a network, DNS, proxy, or TLS problem. Details: {error}"
        );
    }

    if error.is_body() {
        return format!(
            "The request was created, but the upload body could not be sent cleanly to {host}. This usually means the connection was closed while the file was being uploaded. Details: {error}"
        );
    }

    format!("The upload request failed before {host} could answer. Details: {error}")
}

fn upload_response_body_message(
    host: &str,
    status: reqwest::StatusCode,
    error: &reqwest::Error,
) -> String {
    if error.is_timeout() {
        return format!(
            "{host} returned HTTP {status}, but the response body timed out before it could be read."
        );
    }

    if error.is_body() {
        return format!(
            "{host} returned HTTP {status}, but the response body could not be read. This usually means the server closed the connection early. Details: {error}"
        );
    }

    format!("{host} returned HTTP {status}, but reading the response body failed. Details: {error}")
}

#[cfg(windows)]
fn capture_clip_studio_png() -> Result<Vec<u8>, CaptureShareError> {
    windows_capture::capture_clip_studio_png()
}

#[cfg(not(windows))]
fn capture_clip_studio_png() -> Result<Vec<u8>, CaptureShareError> {
    Err(CaptureShareError::UnsupportedPlatform)
}

#[derive(Debug, thiserror::Error)]
pub enum CaptureShareError {
    #[error("Clip Studio Paint window was not found.")]
    WindowNotFound,
    #[error("Clip Studio Paint window is too small or minimized.")]
    InvalidWindowSize,
    #[error("Could not capture the Clip Studio Paint window.")]
    CaptureFailed,
    #[error("Could not encode the screenshot: {0}")]
    Encode(#[from] png::EncodingError),
    #[error("Could not upload the screenshot: {0}")]
    Upload(#[from] reqwest::Error),
    #[error("Could not send the upload request to 0x0.st: {message}")]
    UploadRequest { message: String },
    #[error("Uguu returned HTTP {status}, but reading the response body failed: {message}")]
    UploadResponseBody {
        status: reqwest::StatusCode,
        message: String,
    },
    #[error("Could not create the upload file part: {0}")]
    Mime(#[from] reqwest::header::InvalidHeaderValue),
    #[error("The image host rejected the upload: {0}")]
    UploadRejected(String),
    #[error("Uguu returned HTTP {status} with body: {body}")]
    UploadFailed {
        status: reqwest::StatusCode,
        body: String,
    },
    #[error("Could not access the app cache directory.")]
    AppCacheDir,
    #[error("Could not save the local screenshot: {0}")]
    Io(#[from] std::io::Error),
    #[cfg(not(windows))]
    #[error("Screenshot capture is currently only available on Windows.")]
    UnsupportedPlatform,
    #[error("Could not decode the screenshot before applying the LUT: {0}")]
    ScreenshotDecode(png::DecodingError),
    #[error("Could not decode the screenshot before applying the LUT: {0}")]
    ScreenshotDecodeMessage(String),
    #[error("Screenshot LUT path is enabled but empty.")]
    LutPathMissing,
    #[error("Could not decode the PNG LUT file '{path}': {source}")]
    LutDecode {
        path: String,
        source: png::DecodingError,
    },
    #[error("Could not read the LUT file '{path}': {source}")]
    LutRead {
        path: String,
        source: std::io::Error,
    },
    #[error("The LUT file '{path}' is not valid: {message}")]
    LutParse { path: String, message: String },
}

#[cfg(windows)]
mod windows_capture {
    use super::*;
    use crate::clip_studio::windows::clip_studio_window;
    use std::ffi::c_void;
    use windows_sys::Win32::{
        Foundation::{HWND, RECT},
        Graphics::Gdi::{
            BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits,
            GetWindowDC, ReleaseDC, SelectObject, BITMAPINFO, BI_RGB, DIB_RGB_COLORS, RGBQUAD,
            SRCCOPY,
        },
        Storage::Xps::PrintWindow,
        UI::WindowsAndMessaging::GetWindowRect,
    };

    pub fn capture_clip_studio_png() -> Result<Vec<u8>, CaptureShareError> {
        let hwnd = clip_studio_window().ok_or(CaptureShareError::WindowNotFound)?;
        let mut rect = unsafe { zeroed::<RECT>() };
        if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
            return Err(CaptureShareError::CaptureFailed);
        }

        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        if width <= 8 || height <= 8 {
            return Err(CaptureShareError::InvalidWindowSize);
        }

        let rgba = capture_window_rgba(hwnd, width, height)?;
        encode_png(width as u32, height as u32, &rgba)
    }

    fn capture_window_rgba(
        hwnd: HWND,
        width: i32,
        height: i32,
    ) -> Result<Vec<u8>, CaptureShareError> {
        let window_dc = unsafe { GetWindowDC(hwnd) };
        if window_dc.is_null() {
            return Err(CaptureShareError::CaptureFailed);
        }

        let memory_dc = unsafe { CreateCompatibleDC(window_dc) };
        if memory_dc.is_null() {
            unsafe {
                ReleaseDC(hwnd, window_dc);
            }
            return Err(CaptureShareError::CaptureFailed);
        }

        let bitmap = unsafe { CreateCompatibleBitmap(window_dc, width, height) };
        if bitmap.is_null() {
            unsafe {
                DeleteDC(memory_dc);
                ReleaseDC(hwnd, window_dc);
            }
            return Err(CaptureShareError::CaptureFailed);
        }

        let old_object = unsafe { SelectObject(memory_dc, bitmap) };
        let printed = unsafe { PrintWindow(hwnd, memory_dc, 2) } != 0;
        if !printed {
            unsafe {
                BitBlt(memory_dc, 0, 0, width, height, window_dc, 0, 0, SRCCOPY);
            }
        }

        let mut bitmap_info = BITMAPINFO {
            bmiHeader: unsafe { zeroed() },
            bmiColors: [RGBQUAD {
                rgbBlue: 0,
                rgbGreen: 0,
                rgbRed: 0,
                rgbReserved: 0,
            }],
        };
        bitmap_info.bmiHeader.biSize =
            std::mem::size_of::<windows_sys::Win32::Graphics::Gdi::BITMAPINFOHEADER>() as u32;
        bitmap_info.bmiHeader.biWidth = width;
        bitmap_info.bmiHeader.biHeight = -height;
        bitmap_info.bmiHeader.biPlanes = 1;
        bitmap_info.bmiHeader.biBitCount = 32;
        bitmap_info.bmiHeader.biCompression = BI_RGB;

        let mut bgra = vec![0u8; (width * height * 4) as usize];
        let copied = unsafe {
            GetDIBits(
                memory_dc,
                bitmap,
                0,
                height as u32,
                bgra.as_mut_ptr().cast::<c_void>(),
                &mut bitmap_info,
                DIB_RGB_COLORS,
            )
        };

        unsafe {
            SelectObject(memory_dc, old_object);
            DeleteObject(bitmap);
            DeleteDC(memory_dc);
            ReleaseDC(hwnd, window_dc);
        }

        if copied == 0 {
            return Err(CaptureShareError::CaptureFailed);
        }

        for pixel in bgra.chunks_exact_mut(4) {
            pixel.swap(0, 2);
            pixel[3] = 255;
        }

        Ok(bgra)
    }

    fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, CaptureShareError> {
        let mut output = Cursor::new(Vec::new());
        let mut encoder = png::Encoder::new(&mut output, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(rgba)?;
        drop(writer);
        Ok(output.into_inner())
    }
}
