// 通过 OpenInputDesktop 检测锁屏：屏幕锁定时输入桌面会切换到
// "Winlogon" 桌面，普通进程没有权限打开，调用返回错误。
#[cfg(target_os = "windows")]
pub fn is_locked() -> bool {
    use windows::Win32::System::StationsAndDesktops::{
        CloseDesktop, OpenInputDesktop, DESKTOP_ACCESS_FLAGS, DESKTOP_CONTROL_FLAGS,
    };
    unsafe {
        match OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0), false, DESKTOP_ACCESS_FLAGS(0)) {
            Ok(h) => {
                let _ = CloseDesktop(h);
                false
            }
            Err(_) => true,
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn is_locked() -> bool {
    // macOS / Linux 锁屏检测留作后续。首版仅 Windows 实现，
    // 其他平台始终返回 false，不影响计时器正常工作。
    false
}
