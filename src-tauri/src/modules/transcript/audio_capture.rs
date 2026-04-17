use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

use crossbeam_channel::Sender;

#[cfg(target_os = "windows")]
use std::ffi::c_void;

#[cfg(target_os = "windows")]
use windows::{
    core::{ComInterface, Error as WinError, GUID, HRESULT, Interface},
    Win32::{
        Foundation::HANDLE,
        Media::Audio::{
            eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDevice, IMMDeviceEnumerator,
            MMDeviceEnumerator, AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_EVENTCALLBACK, AUDCLNT_STREAMFLAGS_LOOPBACK, WAVEFORMATEX,
        },
        System::{
            Com::{
                CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL,
                COINIT_MULTITHREADED,
            },
            Threading::{CreateEventW, WaitForSingleObject},
        },
    },
};

const TARGET_CHUNK_MS: u64 = 2_500;

#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples_i16: Vec<i16>,
    pub start_ms: u64,
    pub end_ms: u64,
}

pub struct CaptureHandle {
    stop_flag: Arc<AtomicBool>,
}

impl CaptureHandle {
    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
    }
}

#[cfg(target_os = "windows")]
fn to_string_error(err: WinError) -> String {
    err.to_string()
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct IUnknownVtblCompat {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct IMMDeviceVtblCompat {
    pub base__: IUnknownVtblCompat,
    pub activate: unsafe extern "system" fn(
        *mut c_void,
        *const GUID,
        u32,
        *const c_void,
        *mut *mut c_void,
    ) -> HRESULT,
    pub open_property_store: *const c_void,
    pub get_id: *const c_void,
    pub get_state: *const c_void,
}

#[cfg(target_os = "windows")]
unsafe fn activate_audio_client_from_device(device: &IMMDevice) -> Result<IAudioClient, String> {
    let raw = Interface::as_raw(device) as *mut c_void;
    let vtbl = *(raw as *mut *mut IMMDeviceVtblCompat);

    let mut audio_client_ptr: *mut c_void = std::ptr::null_mut();

    ((*vtbl).activate)(
        raw,
        &IAudioClient::IID as *const GUID,
        CLSCTX_ALL.0 as u32,
        std::ptr::null(),
        &mut audio_client_ptr,
    )
    .ok()
    .map_err(to_string_error)?;

    Ok(IAudioClient::from_raw(audio_client_ptr as _))
}

pub fn start_system_loopback_capture(tx: Sender<AudioChunk>) -> Result<CaptureHandle, String> {
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_thread = stop_flag.clone();

    thread::spawn(move || {
        println!("[audio] loopback thread started");

        if let Err(err) = run_system_loopback_capture(stop_flag_thread, tx) {
            eprintln!("[audio] FATAL ERROR: {}", err);
        }

        println!("[audio] loopback thread ended");
    });

    Ok(CaptureHandle { stop_flag })
}

#[cfg(target_os = "windows")]
fn run_system_loopback_capture(
    stop_flag: Arc<AtomicBool>,
    tx: Sender<AudioChunk>,
) -> Result<(), String> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).map_err(to_string_error)?;
    }

    let result = (|| unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(to_string_error)?;

        let device = enumerator
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .map_err(to_string_error)?;

        let audio_client: IAudioClient = activate_audio_client_from_device(&device)?;

        let mix_format_ptr = audio_client.GetMixFormat().map_err(to_string_error)?;
        let mix_format = *(mix_format_ptr as *const WAVEFORMATEX);

        let format_tag = mix_format.wFormatTag;
        let input_channels = mix_format.nChannels;
        let input_sample_rate = mix_format.nSamplesPerSec;
        let bits_per_sample = mix_format.wBitsPerSample;

        println!(
            "[audio] format tag={}, channels={}, sample_rate={}, bits_per_sample={}",
            format_tag, input_channels, input_sample_rate, bits_per_sample
        );

        let event: HANDLE = CreateEventW(None, false, false, None).map_err(to_string_error)?;

        audio_client
            .Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_LOOPBACK | AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
                2_000_000,
                0,
                mix_format_ptr,
                None,
            )
            .map_err(to_string_error)?;

        audio_client
            .SetEventHandle(event)
            .map_err(to_string_error)?;

        let capture_client: IAudioCaptureClient = audio_client
            .GetService::<IAudioCaptureClient>()
            .map_err(to_string_error)?;

        audio_client.Start().map_err(to_string_error)?;

        let mut mono_i16_buffer: Vec<i16> = Vec::new();
        let mut chunk_start_ms = 0u64;

        while !stop_flag.load(Ordering::SeqCst) {
            let _ = WaitForSingleObject(event, 200);

            let mut next_packet_size = capture_client
                .GetNextPacketSize()
                .map_err(to_string_error)?;

            while next_packet_size > 0 {
                let mut data_ptr = std::ptr::null_mut();
                let mut num_frames = 0u32;
                let mut flags = 0u32;

                capture_client
                    .GetBuffer(&mut data_ptr, &mut num_frames, &mut flags, None, None)
                    .map_err(to_string_error)?;

                let sample_count = (num_frames * input_channels as u32) as usize;

                if (flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32) != 0 {
                    mono_i16_buffer.extend(std::iter::repeat_n(0i16, num_frames as usize));
                } else {
                    let f32_samples =
                        std::slice::from_raw_parts(data_ptr as *const f32, sample_count);

                    mono_i16_buffer.extend(downmix_f32_to_mono_i16(
                        f32_samples,
                        input_channels,
                    ));
                }

                capture_client
                    .ReleaseBuffer(num_frames)
                    .map_err(to_string_error)?;

                next_packet_size = capture_client
                    .GetNextPacketSize()
                    .map_err(to_string_error)?;
            }

            let current_ms = mono_samples_to_ms(mono_i16_buffer.len(), input_sample_rate);

            if current_ms >= TARGET_CHUNK_MS {
                let samples = std::mem::take(&mut mono_i16_buffer);

                let chunk = AudioChunk {
                    sample_rate: input_sample_rate,
                    channels: 1,
                    samples_i16: samples,
                    start_ms: chunk_start_ms,
                    end_ms: chunk_start_ms + current_ms,
                };

                println!(
                    "[audio] sending chunk: {}-{} ms, samples={}",
                    chunk.start_ms,
                    chunk.end_ms,
                    chunk.samples_i16.len()
                );

                tx.send(chunk).map_err(|e| e.to_string())?;
                chunk_start_ms += current_ms;
            }
        }

        if !mono_i16_buffer.is_empty() {
            let current_ms = mono_samples_to_ms(mono_i16_buffer.len(), input_sample_rate);

            let chunk = AudioChunk {
                sample_rate: input_sample_rate,
                channels: 1,
                samples_i16: mono_i16_buffer,
                start_ms: chunk_start_ms,
                end_ms: chunk_start_ms + current_ms,
            };

            println!(
                "[audio] sending final chunk: {}-{} ms, samples={}",
                chunk.start_ms,
                chunk.end_ms,
                chunk.samples_i16.len()
            );

            let _ = tx.send(chunk);
        }

        let _ = audio_client.Stop();
        CoTaskMemFree(Some(mix_format_ptr as _));
        Ok(())
    })();

    unsafe {
        CoUninitialize();
    }

    result
}

#[cfg(not(target_os = "windows"))]
fn run_system_loopback_capture(
    _stop_flag: Arc<AtomicBool>,
    _tx: Sender<AudioChunk>,
) -> Result<(), String> {
    Err("System loopback capture is only implemented for Windows".into())
}

fn mono_samples_to_ms(sample_count: usize, sample_rate: u32) -> u64 {
    ((sample_count as f64 / sample_rate as f64) * 1000.0) as u64
}

fn downmix_f32_to_mono_i16(samples: &[f32], channels: u16) -> Vec<i16> {
    if channels <= 1 {
        return samples
            .iter()
            .map(|s| float_to_i16(*s))
            .collect();
    }

    let ch = channels as usize;
    let mut mono = Vec::with_capacity(samples.len() / ch);

    for frame in samples.chunks_exact(ch) {
        let avg = frame.iter().copied().sum::<f32>() / channels as f32;
        mono.push(float_to_i16(avg));
    }

    mono
}

fn float_to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
}