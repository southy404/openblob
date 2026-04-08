use crate::modules::command_router::CompanionAction;

pub fn resolve_app_action(
    action: CompanionAction,
    active_app: &str,
) -> Option<CompanionAction> {
    let app = active_app.to_lowercase();

    if is_browser(&app) {
        return resolve_browser_action(action);
    }

    if app.contains("mspaint") || app.contains("paint") {
        return resolve_paint_action(action);
    }

    if app.contains("notepad") {
        return resolve_notepad_action(action);
    }

    if app.contains("calc") {
        return resolve_calculator_action(action);
    }

    None
}

fn is_browser(app: &str) -> bool {
    app.contains("chrome") || app.contains("edge") || app.contains("firefox")
}

fn resolve_paint_action(action: CompanionAction) -> Option<CompanionAction> {
    match action {
        CompanionAction::Save => Some(CompanionAction::KeyCombo(vec!["ctrl", "s"])),
        CompanionAction::SaveAs => Some(CompanionAction::KeyCombo(vec!["ctrl", "shift", "s"])),
        CompanionAction::OpenFile => Some(CompanionAction::KeyCombo(vec!["ctrl", "o"])),
        CompanionAction::NewFile => Some(CompanionAction::KeyCombo(vec!["ctrl", "n"])),
        CompanionAction::Close | CompanionAction::CloseApp => {
            Some(CompanionAction::KeyCombo(vec!["alt", "f4"]))
        }
        _ => None,
    }
}

fn resolve_notepad_action(action: CompanionAction) -> Option<CompanionAction> {
    match action {
        CompanionAction::Save => Some(CompanionAction::KeyCombo(vec!["ctrl", "s"])),
        CompanionAction::SaveAs => Some(CompanionAction::KeyCombo(vec!["ctrl", "shift", "s"])),
        CompanionAction::OpenFile => Some(CompanionAction::KeyCombo(vec!["ctrl", "o"])),
        CompanionAction::NewFile => Some(CompanionAction::KeyCombo(vec!["ctrl", "n"])),
        CompanionAction::Close | CompanionAction::CloseApp => {
            Some(CompanionAction::KeyCombo(vec!["alt", "f4"]))
        }
        _ => None,
    }
}

fn resolve_browser_action(action: CompanionAction) -> Option<CompanionAction> {
    match action {
        CompanionAction::NewTab => Some(CompanionAction::KeyCombo(vec!["ctrl", "t"])),
        CompanionAction::CloseTab => Some(CompanionAction::KeyCombo(vec!["ctrl", "w"])),
        CompanionAction::NewWindow => Some(CompanionAction::KeyCombo(vec!["ctrl", "n"])),
        CompanionAction::Incognito => Some(CompanionAction::KeyCombo(vec!["ctrl", "shift", "n"])),
        CompanionAction::Reload => Some(CompanionAction::KeyCombo(vec!["ctrl", "r"])),
        CompanionAction::Close | CompanionAction::CloseApp => {
            Some(CompanionAction::KeyCombo(vec!["alt", "f4"]))
        }
        CompanionAction::YouTubeNextVideo => Some(CompanionAction::KeyCombo(vec!["shift", "n"])),
        CompanionAction::YouTubeSeekForward => Some(CompanionAction::KeyPress("l")),
        CompanionAction::YouTubeSeekBackward => Some(CompanionAction::KeyPress("j")),
        CompanionAction::MediaPlayPause => Some(CompanionAction::KeyPress("k")),
        _ => None,
    }
}

fn resolve_calculator_action(action: CompanionAction) -> Option<CompanionAction> {
    match action {
        CompanionAction::InsertText(text) => Some(CompanionAction::InsertText(text)),
        CompanionAction::Confirm => Some(CompanionAction::KeyPress("enter")),
        CompanionAction::Clear => Some(CompanionAction::KeyPress("escape")),
        CompanionAction::Close | CompanionAction::CloseApp => {
            Some(CompanionAction::KeyCombo(vec!["alt", "f4"]))
        }
        _ => None,
    }
}