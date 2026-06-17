#[derive(Clone, Debug, Default)]
pub struct ClipStudioDetection {
    pub running: bool,
    pub focused: bool,
    pub document_title: Option<String>,
}

#[cfg(windows)]
pub fn detect_clip_studio() -> ClipStudioDetection {
    let processes = windows::clip_studio_processes();
    let running = !processes.is_empty();
    let foreground = windows::foreground_window();
    let focused = foreground
        .as_ref()
        .map(|window| processes.iter().any(|process| process.pid == window.pid))
        .unwrap_or(false);

    let document_title = foreground
        .filter(|_| focused)
        .and_then(|window| sanitize_window_title(&window.title));

    ClipStudioDetection {
        running,
        focused,
        document_title,
    }
}

#[cfg(not(windows))]
pub fn detect_clip_studio() -> ClipStudioDetection {
    ClipStudioDetection::default()
}

fn sanitize_window_title(title: &str) -> Option<String> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return None;
    }

    let without_suffix = trimmed
        .strip_suffix(" - CLIP STUDIO PAINT")
        .or_else(|| trimmed.strip_suffix(" - Clip Studio Paint"))
        .unwrap_or(trimmed)
        .trim();

    if without_suffix.is_empty() {
        None
    } else {
        Some(without_suffix.to_string())
    }
}

#[cfg(windows)]
pub(crate) mod windows {
    use std::{
        ffi::c_void,
        mem::{size_of, zeroed},
    };
    use windows_sys::Win32::{
        Foundation::{CloseHandle, BOOL, HWND, INVALID_HANDLE_VALUE, LPARAM},
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
        UI::WindowsAndMessaging::{
            EnumWindows, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
            GetWindowThreadProcessId, IsWindowVisible,
        },
    };

    const CLIP_STUDIO_PROCESS_NAMES: &[&str] = &["CLIPStudioPaint.exe", "CLIPStudioPaintApp.exe"];

    #[derive(Clone, Debug)]
    pub struct ProcessInfo {
        pub pid: u32,
    }

    #[derive(Clone, Debug)]
    pub struct WindowInfo {
        pub pid: u32,
        pub title: String,
    }

    pub fn clip_studio_processes() -> Vec<ProcessInfo> {
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if snapshot == INVALID_HANDLE_VALUE {
            return Vec::new();
        }

        let mut entry = unsafe { zeroed::<PROCESSENTRY32W>() };
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;

        let mut processes = Vec::new();
        let mut has_entry = unsafe { Process32FirstW(snapshot, &mut entry) } != 0;

        while has_entry {
            let process_name = utf16z_to_string(&entry.szExeFile);
            if is_clip_studio_process(&process_name) {
                processes.push(ProcessInfo {
                    pid: entry.th32ProcessID,
                });
            }
            has_entry = unsafe { Process32NextW(snapshot, &mut entry) } != 0;
        }

        unsafe {
            CloseHandle(snapshot);
        }

        processes
    }

    pub fn foreground_window() -> Option<WindowInfo> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.is_null() {
            return None;
        }

        let title = window_title(hwnd)?;
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut pid);
        }

        if pid == 0 {
            return None;
        }

        Some(WindowInfo { pid, title })
    }

    pub fn clip_studio_window() -> Option<HWND> {
        let pids = clip_studio_process_ids();
        if pids.is_empty() {
            return None;
        }

        let mut matches = Vec::<HWND>::new();
        let mut context = WindowSearchContext {
            pids: &pids,
            matches: &mut matches,
        };

        unsafe {
            EnumWindows(
                Some(enum_windows_callback),
                (&mut context as *mut WindowSearchContext).cast::<c_void>() as LPARAM,
            );
        }

        matches.into_iter().next()
    }

    fn clip_studio_process_ids() -> Vec<u32> {
        clip_studio_processes()
            .into_iter()
            .map(|process| process.pid)
            .collect()
    }

    struct WindowSearchContext<'a> {
        pids: &'a [u32],
        matches: &'a mut Vec<HWND>,
    }

    unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let context = &mut *(lparam as *mut WindowSearchContext);
        if IsWindowVisible(hwnd) == 0 || window_title(hwnd).is_none() {
            return 1;
        }

        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if context.pids.contains(&pid) {
            context.matches.push(hwnd);
            return 0;
        }

        1
    }

    fn window_title(hwnd: HWND) -> Option<String> {
        let length = unsafe { GetWindowTextLengthW(hwnd) };
        if length <= 0 {
            return None;
        }

        let mut buffer = vec![0u16; length as usize + 1];
        let copied = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
        if copied <= 0 {
            return None;
        }

        Some(String::from_utf16_lossy(&buffer[..copied as usize]))
    }

    fn is_clip_studio_process(process_name: &str) -> bool {
        CLIP_STUDIO_PROCESS_NAMES
            .iter()
            .any(|candidate| process_name.eq_ignore_ascii_case(candidate))
    }

    fn utf16z_to_string(buffer: &[u16]) -> String {
        let end = buffer
            .iter()
            .position(|char| *char == 0)
            .unwrap_or(buffer.len());
        String::from_utf16_lossy(&buffer[..end])
    }
}
