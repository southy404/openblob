use super::types::{CapabilityContext, CapabilityDescriptor, CapabilityId, PermissionLevel};

pub const CAP_BROWSER_SEARCH_GOOGLE: CapabilityId = "browser.search_google";
pub const CAP_BROWSER_SEARCH_YOUTUBE: CapabilityId = "browser.search_youtube";
pub const CAP_SYSTEM_OPEN_APP: CapabilityId = "system.open_app";
pub const CAP_VISION_CAPTURE_SCREEN: CapabilityId = "vision.capture_screen";
pub const CAP_MEDIA_PLAY_PAUSE: CapabilityId = "media.play_pause";

const ANY: &[CapabilityContext] = &[CapabilityContext::Any];
const BROWSER_OR_ANY: &[CapabilityContext] = &[CapabilityContext::Browser, CapabilityContext::Any];
const DESKTOP_OR_ANY: &[CapabilityContext] = &[CapabilityContext::Desktop, CapabilityContext::Any];
const MEDIA_OR_ANY: &[CapabilityContext] = &[CapabilityContext::Media, CapabilityContext::Any];

pub static CAPABILITIES: &[CapabilityDescriptor] = &[
    CapabilityDescriptor {
        id: CAP_BROWSER_SEARCH_GOOGLE,
        title: "Google Search",
        description: "Search Google in the controlled browser.",
        permission: PermissionLevel::Safe,
        contexts: BROWSER_OR_ANY,
        unstable: false,
    },
    CapabilityDescriptor {
        id: CAP_BROWSER_SEARCH_YOUTUBE,
        title: "YouTube Search",
        description: "Search YouTube in the controlled browser.",
        permission: PermissionLevel::Safe,
        contexts: BROWSER_OR_ANY,
        unstable: false,
    },
    CapabilityDescriptor {
        id: CAP_SYSTEM_OPEN_APP,
        title: "Open Application",
        description: "Open a local application or known target.",
        permission: PermissionLevel::Safe,
        contexts: DESKTOP_OR_ANY,
        unstable: false,
    },
    CapabilityDescriptor {
        id: CAP_VISION_CAPTURE_SCREEN,
        title: "Capture Screen",
        description: "Capture a screenshot or start the snip flow.",
        permission: PermissionLevel::Confirm,
        contexts: ANY,
        unstable: true,
    },
    CapabilityDescriptor {
        id: CAP_MEDIA_PLAY_PAUSE,
        title: "Play / Pause Media",
        description: "Send media play/pause key event to the system.",
        permission: PermissionLevel::Safe,
        contexts: MEDIA_OR_ANY,
        unstable: false,
    },
];

pub fn all_capabilities() -> &'static [CapabilityDescriptor] {
    CAPABILITIES
}

pub fn find_capability(id: &str) -> Option<&'static CapabilityDescriptor> {
    CAPABILITIES.iter().find(|cap| cap.id == id)
}