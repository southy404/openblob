use super::extract::extract_timer_minutes;
use super::types::CompanionAction;
use crate::modules::i18n::command_locale::command_locale;

fn matches_any_contains(input: &str, phrases: &[String]) -> bool {
    phrases.iter().any(|p| !p.trim().is_empty() && input.contains(p))
}

fn matches_any_exact(input: &str, phrases: &[String]) -> bool {
    let trimmed = input.trim();
    phrases.iter().any(|p| trimmed == p.trim())
}

pub fn parse_utility_command(normalized: &str) -> Option<CompanionAction> {
    let locale = command_locale();

    if matches_any_exact(normalized, &locale.current_time_phrases) {
        return Some(CompanionAction::CurrentTime);
    }

    if matches_any_exact(normalized, &locale.current_date_phrases) {
        return Some(CompanionAction::CurrentDate);
    }

    if matches_any_contains(normalized, &locale.coin_flip_phrases) {
        return Some(CompanionAction::CoinFlip);
    }

    if matches_any_contains(normalized, &locale.roll_dice_phrases) {
        return Some(CompanionAction::RollDice);
    }

    if matches_any_contains(normalized, &locale.timer_phrases) {
        let minutes = extract_timer_minutes(normalized).unwrap_or(5);
        return Some(CompanionAction::SetTimer { minutes });
    }

    if matches_any_contains(normalized, &locale.screenshot_words) {
        return Some(CompanionAction::TakeScreenshot);
    }

    None
}