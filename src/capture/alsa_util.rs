use std::collections::HashMap;
use std::ffi::CString;

#[cfg(target_os = "linux")]
pub fn get_alsa_hint_for(name: &str) -> Option<String> {
    let hints = get_alsa_hints();
    hints.get(name).cloned()
}

#[cfg(target_os = "linux")]
pub fn get_alsa_hints() -> HashMap<String, String> {
    use alsa::Direction;
    use alsa::device_name::HintIter;

    let mut hints = HashMap::new();

    let iface = CString::new("pcm").unwrap();
    let hint_iter = HintIter::new(None, &iface).expect("Failed to get ALSA hints");

    for hint in hint_iter {
        let name = hint.name.expect("Failed to get hint name");
        let desc = hint.desc.expect("Failed to get hint description");

        if let Some(direction) = hint.direction {
            if direction != Direction::Capture {
                continue;
            }
        }

        hints.insert(name.to_string(), desc.to_string());
    }

    hints
}
