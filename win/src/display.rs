//! Queries the highest active display refresh rate on the machine so the frame-rate menu can
//! offer the user's native high-refresh tier (e.g. 120 / 144 / 165 / 240 fps).

#[cfg(windows)]
mod imp {
    use std::mem;

    use windows::core::PCWSTR;
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayDevicesW, EnumDisplaySettingsW, DEVMODEW, DISPLAY_DEVICEW,
        DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, ENUM_CURRENT_SETTINGS,
    };

    /// Highest refresh rate (in Hz) across all attached displays, falling back to 60.
    pub fn max_refresh_hz() -> u32 {
        let mut max_hz = 0u32;
        unsafe {
            let mut index = 0u32;
            loop {
                let mut dev: DISPLAY_DEVICEW = mem::zeroed();
                dev.cb = mem::size_of::<DISPLAY_DEVICEW>() as u32;
                if !EnumDisplayDevicesW(PCWSTR::null(), index, &mut dev, 0).as_bool() {
                    break;
                }
                index += 1;
                if (dev.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP).0 == 0 {
                    continue;
                }
                let mut mode: DEVMODEW = mem::zeroed();
                mode.dmSize = mem::size_of::<DEVMODEW>() as u16;
                if EnumDisplaySettingsW(
                    PCWSTR(dev.DeviceName.as_ptr()),
                    ENUM_CURRENT_SETTINGS,
                    &mut mode,
                )
                .as_bool()
                {
                    let hz = mode.dmDisplayFrequency;
                    if (30..=1000).contains(&hz) {
                        max_hz = max_hz.max(hz);
                    }
                }
            }
            if max_hz == 0 {
                let mut mode: DEVMODEW = mem::zeroed();
                mode.dmSize = mem::size_of::<DEVMODEW>() as u16;
                if EnumDisplaySettingsW(PCWSTR::null(), ENUM_CURRENT_SETTINGS, &mut mode).as_bool() {
                    let hz = mode.dmDisplayFrequency;
                    if (30..=1000).contains(&hz) {
                        max_hz = hz;
                    }
                }
            }
        }
        if max_hz >= 30 { max_hz } else { 60 }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn max_refresh_hz() -> u32 {
        60
    }
}

pub use imp::max_refresh_hz;
