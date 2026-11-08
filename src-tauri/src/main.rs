#![windows_subsystem = "windows"]

fn log_main(msg: &str) {
    aria2_app_lib::log_line("main", msg);
}

#[cfg(windows)]
fn check_single_instance() -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
    use windows_sys::Win32::System::Threading::{
        CreateEventW, CreateMutexW, OpenEventW, SetEvent, EVENT_MODIFY_STATE,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        AllowSetForegroundWindow, BringWindowToTop, FindWindowW, SetForegroundWindow, ShowWindow,
        ASFW_ANY, SW_RESTORE, SW_SHOW,
    };

    let mutex_name: Vec<u16> = "Local\\KestrelAppMutex\0".encode_utf16().collect();
    let _hmutex = unsafe { CreateMutexW(std::ptr::null(), 1, mutex_name.as_ptr()) };

    let event_name: Vec<u16> = "Local\\KestrelAppRestoreEvent\0".encode_utf16().collect();

    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        log_main("Secondary instance detected. Signaling primary instance to restore window...");

        unsafe {
            // Grant foreground activation permission to the existing app
            let _ = AllowSetForegroundWindow(ASFW_ANY);

            // Signal the primary instance via Named Event
            let mut event = OpenEventW(EVENT_MODIFY_STATE, 0, event_name.as_ptr());
            if event.is_null() {
                std::thread::sleep(std::time::Duration::from_millis(100));
                event = OpenEventW(EVENT_MODIFY_STATE, 0, event_name.as_ptr());
            }

            if !event.is_null() {
                SetEvent(event);
                CloseHandle(event);
                log_main("Restore event signaled successfully.");
            } else {
                log_main("Could not open restore event, attempting fallback window restore...");
                // Fallback direct window restore
                let win_title: Vec<u16> = "Kestrel Download Manager\0".encode_utf16().collect();
                let hwnd = FindWindowW(std::ptr::null(), win_title.as_ptr());
                if !hwnd.is_null() {
                    ShowWindow(hwnd, SW_SHOW);
                    ShowWindow(hwnd, SW_RESTORE);
                    BringWindowToTop(hwnd);
                    SetForegroundWindow(hwnd);
                    log_main("Restored via fallback FindWindowW.");
                }
            }
        }
        return false;
    }

    // Primary instance: create the event so it exists immediately
    unsafe {
        let _hevent = CreateEventW(std::ptr::null(), 0, 0, event_name.as_ptr());
    }

    true
}

fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        log_main(&format!("PANIC: {:?}", panic_info));
    }));

    #[cfg(windows)]
    if !check_single_instance() {
        log_main("Exiting secondary instance immediately.");
        return;
    }

    log_main("Primary instance starting...");

    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;
        let app_id: Vec<u16> = "Kestrel.DownloadManager.App\0".encode_utf16().collect();
        let _ = SetCurrentProcessExplicitAppUserModelID(app_id.as_ptr());
    }

    aria2_app_lib::run();
    log_main("App run() finished normally.");
}
