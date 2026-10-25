use crate::session::LoginDevice;

/// How the client page's device check came back (see device.js).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceCheck {
    Missing,
    Sealed,
    Broken,
}

/// An operating system, as a user agent or a page names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Os {
    Windows,
    Mac,
    Ios,
    Android,
    Chromebook,
    Linux,
}

/// The operating system a user agent names.
fn ua_os(ua: &str) -> Option<Os> {
    if ["iPhone", "iPad", "iPod"].iter().any(|w| ua.contains(w)) {
        Some(Os::Ios)
    } else if ua.contains("Android") {
        Some(Os::Android)
    } else if ua.contains("Windows") {
        Some(Os::Windows)
    } else if ua.contains("CrOS") {
        Some(Os::Chromebook)
    } else if ua.contains("Macintosh") || ua.contains("Mac OS X") {
        Some(Os::Mac)
    } else if ua.contains("Linux") || ua.contains("X11") {
        Some(Os::Linux)
    } else {
        None
    }
}

/// The rendering engine a user agent names: every iOS browser is WebKit.
fn ua_engine(ua: &str) -> Option<&'static str> {
    if ua_os(ua) == Some(Os::Ios) {
        Some("webkit")
    } else if ua.contains("Firefox/") {
        Some("gecko")
    } else if ["Chrome/", "Chromium/", "Edg/", "OPR/"]
        .iter()
        .any(|w| ua.contains(w))
    {
        Some("blink")
    } else if ua.contains("Safari/") && ua.contains("Version/") {
        Some("webkit")
    } else {
        None
    }
}

/// Whether `navigator.platform` fits the system the user agent names. An
/// iPad asking for desktop sites says Mac in both.
fn platform_fits(os: Os, js_platform: &str) -> bool {
    let p = js_platform;
    if p.is_empty() {
        return true;
    }
    match os {
        Os::Windows => p.starts_with("Win"),
        Os::Mac => p.starts_with("Mac"),
        Os::Ios => p.starts_with("iP") || p.starts_with("Mac"),
        Os::Android | Os::Chromebook | Os::Linux => {
            p.starts_with("Linux") || p.starts_with("Android") || p.starts_with("CrOS")
        }
    }
}

/// Whether the graphics card fits the system the user agent names, where
/// the card's name gives its system away: Direct3D is Windows', Apple's
/// chips and Metal are Apple's, Adreno and Mali are phones'.
fn graphics_fit(os: Os, gpu: &str) -> bool {
    if gpu.contains("Direct3D") {
        os == Os::Windows
    } else if gpu.contains("Apple") || gpu.contains("Metal") {
        matches!(os, Os::Mac | Os::Ios)
    } else if gpu.contains("Adreno") || gpu.contains("Mali") {
        matches!(os, Os::Android | Os::Chromebook | Os::Linux)
    } else {
        true
    }
}

/// Whether the graphics are drawn in software: a virtual machine, a
/// headless browser, or a browser with hardware acceleration turned off.
pub fn software_graphics(gpu: &str) -> bool {
    ["SwiftShader", "llvmpipe", "Software", "softpipe"]
        .iter()
        .any(|w| gpu.contains(w))
}

impl LoginDevice {
    /// How the web client's device check and its browser's claims
    /// disagree, or that the check was missing or forged: empty when all is
    /// as a browser would have it. Nothing for the desktop client, which
    /// has no check.
    pub fn tampering(&self) -> Vec<&'static str> {
        let Some(check) = self.check.filter(|_| self.web) else {
            return Vec::new();
        };
        match check {
            DeviceCheck::Missing => return vec!["no_check"],
            DeviceCheck::Broken => return vec!["bad_seal"],
            DeviceCheck::Sealed => {}
        }
        let mut found = Vec::new();
        if self.agent_differs {
            found.push("user_agent");
        }
        let ua = self.user_agent.as_deref().unwrap_or("");
        let engine = self.engine.as_deref().filter(|e| *e != "other");
        if let (Some(claimed), Some(measured)) = (ua_engine(ua), engine)
            && claimed != measured
        {
            found.push("engine");
        }
        if let Some(os) = ua_os(ua) {
            if !platform_fits(os, self.js_platform.as_deref().unwrap_or("")) {
                found.push("platform");
            }
            if !graphics_fit(os, self.gpu.as_deref().unwrap_or("")) {
                found.push("graphics");
            }
        }
        found
    }

    /// [`Self::tampering`] as stored: `None` for the desktop client.
    pub(crate) fn tamper_column(&self) -> Option<Vec<String>> {
        (self.web && self.check.is_some())
            .then(|| self.tampering().into_iter().map(str::to_owned).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHROME_WIN: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";
    const FIREFOX_MAC: &str =
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 14.5; rv:131.0) Gecko/20100101 Firefox/131.0";
    const CHROME_IOS: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/130.0 Mobile/15E148 Safari/604.1";
    const SAFARI_MAC: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15";

    fn web(ua: &str, engine: &str, platform: &str, gpu: &str) -> LoginDevice {
        LoginDevice {
            web: true,
            user_agent: Some(ua.into()),
            engine: Some(engine.into()),
            js_platform: Some(platform.into()),
            gpu: Some(gpu.into()),
            check: Some(DeviceCheck::Sealed),
            ..LoginDevice::default()
        }
    }

    #[test]
    fn real_browsers_agree_with_themselves() {
        let d3d = "ANGLE (NVIDIA, NVIDIA GeForce RTX 3070 Direct3D11 vs_5_0 ps_5_0, D3D11)";
        for device in [
            web(CHROME_WIN, "blink", "Win32", d3d),
            web(FIREFOX_MAC, "gecko", "MacIntel", "Apple M1"),
            web(CHROME_IOS, "webkit", "iPhone", "Apple GPU"),
            web(SAFARI_MAC, "webkit", "MacIntel", "Apple GPU"),
            web(CHROME_WIN, "other", "", ""),
        ] {
            assert!(device.tampering().is_empty(), "{device:?}");
        }
        // The desktop client has no check to fail.
        assert!(LoginDevice::default().tampering().is_empty());
        assert_eq!(LoginDevice::default().tamper_column(), None);
    }

    #[test]
    fn a_faked_user_agent_gives_itself_away() {
        // A user agent switcher on Firefox for Windows, claiming Chrome on a Mac.
        let ua = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 Chrome/141.0 Safari/537.36";
        let device = LoginDevice {
            agent_differs: true,
            ..web(ua, "gecko", "Win32", "ANGLE (Intel, Direct3D11)")
        };
        assert_eq!(
            device.tampering(),
            ["user_agent", "engine", "platform", "graphics"]
        );
        let missing = LoginDevice {
            check: Some(DeviceCheck::Missing),
            ..LoginDevice::default()
        };
        assert!(missing.tampering().is_empty(), "not the web client");
        let missing = LoginDevice {
            web: true,
            ..missing
        };
        assert_eq!(missing.tampering(), ["no_check"]);
        let forged = LoginDevice {
            check: Some(DeviceCheck::Broken),
            ..web(CHROME_WIN, "blink", "Win32", "")
        };
        assert_eq!(forged.tamper_column(), Some(vec!["bad_seal".to_owned()]));
    }

    #[test]
    fn software_graphics_are_told_apart() {
        assert!(software_graphics(
            "ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero)), SwiftShader driver)"
        ));
        assert!(software_graphics("llvmpipe (LLVM 15.0.7, 256 bits)"));
        assert!(!software_graphics("Apple M2"));
    }
}
