pub const OPEN_WORDS: &[&str] = &["open", "oeffne", "oeffnen", "starte", "start", "launch", "run"];
pub const CLOSE_WORDS: &[&str] = &["close", "schliess", "schliesse", "beenden", "exit", "quit"];
pub const BROWSER_WORDS: &[&str] = &["browser", "web", "website", "chrome", "edge", "online"];
pub const GOOGLE_WORDS: &[&str] = &["google", "googel", "gogle"];
pub const YOUTUBE_WORDS: &[&str] = &["youtube", "youtub", "jutube", "jutub", "yt"];
pub const WEATHER_WORDS: &[&str] = &[
    "wetter", "weather", "temperatur", "temperature", "regen", "rain", "sun", "sonne", "forecast",
];
pub const TIME_WORDS: &[&str] = &["uhr", "uhrzeit", "spaet", "spät", "zeit", "time"];
pub const DATE_WORDS: &[&str] = &["datum", "date", "heute", "tag", "today"];
pub const EXPLAIN_WORDS: &[&str] = &["erklaer", "erklaere", "explain", "meaning", "bedeutet", "mean"];
pub const VOLUME_UP_WORDS: &[&str] = &["lauter", "louder", "increase", "up", "hoch"];
pub const VOLUME_DOWN_WORDS: &[&str] = &["leiser", "quieter", "down", "lower", "runter", "reduce"];
pub const VOLUME_WORDS: &[&str] = &["lautstaerke", "volume", "sound", "ton", "audio"];
pub const MUTE_WORDS: &[&str] = &["mute", "stumm", "silent", "silence", "aus"];
pub const UNMUTE_WORDS: &[&str] = &["unmute", "an", "wieder", "restore"];
pub const PAUSE_WORDS: &[&str] = &["pause", "stop", "pausieren", "hold"];
pub const NEXT_WORDS: &[&str] = &["next", "naechster", "weiter", "skip"];
pub const PREV_WORDS: &[&str] = &["previous", "prev", "zurueck", "back", "vorheriger"];
pub const SAVE_WORDS: &[&str] = &["save", "speichern"];
pub const SAVE_AS_WORDS: &[&str] = &["saveas", "save as", "speichern unter"];
pub const OPEN_FILE_WORDS: &[&str] = &["open file", "datei oeffnen", "file open"];
pub const NEW_FILE_WORDS: &[&str] = &["new file", "neu", "new", "neue datei"];
pub const UNDO_WORDS: &[&str] = &["undo", "rueckgaengig", "zuruecknehmen"];
pub const REDO_WORDS: &[&str] = &["redo", "wiederholen"];
pub const TAB_CLOSE_WORDS: &[&str] = &["close tab", "tab schliessen", "tab close"];
pub const TAB_NEW_WORDS: &[&str] = &["new tab", "neuer tab"];
pub const WINDOW_NEW_WORDS: &[&str] = &["new window", "neues fenster"];
pub const INCOGNITO_WORDS: &[&str] = &["incognito", "inkognito", "private window", "privates fenster"];
pub const RELOAD_WORDS: &[&str] = &["reload", "neu laden", "refresh"];
pub const YT_NEXT_WORDS: &[&str] = &["next video", "naechstes video", "video weiter"];
pub const YT_FORWARD_WORDS: &[&str] = &["vorspulen", "forward", "skip ahead"];
pub const YT_BACK_WORDS: &[&str] = &["zurueckspulen", "rewind", "backward"];
pub const CLICK_WORDS: &[&str] = &["click", "klick", "klicke", "oeffne link", "open link"];
pub const PLAY_WORDS: &[&str] = &["play", "spiele", "abspielen"];
pub const RESULT_WORDS: &[&str] = &["result", "ergebnis", "suchergebnis", "video"];
pub const BACK_WORDS: &[&str] = &["zurueck", "go back", "back", "eine seite zurueck"];
pub const FORWARD_WORDS: &[&str] = &["forward", "weiter", "vor", "go forward"];
pub const SCROLL_DOWN_WORDS: &[&str] = &["scroll runter", "runter scrollen", "scroll down"];
pub const SCROLL_UP_WORDS: &[&str] = &["scroll hoch", "hoch scrollen", "scroll up"];
pub const TYPE_WORDS: &[&str] = &["tippe", "type", "schreibe", "enter text"];
pub const SUBMIT_WORDS: &[&str] = &["submit", "abschicken", "absenden", "drueck enter"];
pub const CONTEXT_WORDS: &[&str] = &["wo bin ich", "seitenkontext", "browser context", "was ist auf der seite"];
pub const SCREENSHOT_WORDS: &[&str] = &[
    "screenshot",
    "screen shot",
    "snip",
    "snapshot",
    "capture",
    "screen capture",
    "take screenshot",
    "take a screenshot",
    "make screenshot",
    "mach screenshot",
    "mach einen screenshot",
    "mach ein screenshot",
    "mach einen screen",
    "bildschirmfoto",
    "bildschirm foto",
    "aufnahme",
    "screenie",
    "snipping",
];

pub const KNOWN_TARGETS: &[(&str, &[&str])] = &[
    ("discord", &["discord", "discrod", "discort", "disord"]),
    ("spotify", &["spotify", "spotfy", "spoti"]),
    ("youtube", &["youtube", "youtub", "jutube", "yt"]),
    ("google", &["google", "googel", "gogle"]),
    ("chrome", &["chrome", "chrom"]),
    ("edge", &["edge", "msedge", "microsoftedge"]),
    ("twitch", &["twitch", "twuicth", "twich", "twtich"]),
    ("github", &["github", "git hub"]),
    ("reddit", &["reddit", "redit"]),
    ("paint", &["paint", "mspaint"]),
    ("notepad", &["notepad", "editor", "texteditor"]),
    ("explorer", &["explorer", "fileexplorer", "dateiexplorer"]),
    ("calc", &["calc", "calculator", "rechner", "taschenrechner"]),
    ("taskmgr", &["taskmanager", "taskmgr"]),
    ("settings", &["settings", "einstellungen"]),
    ("gmail", &["gmail", "googlemail", "mail"]),
    ("steam", &["steam", "steem", "steeeam", "stim"]),
    ("fl studio", &["fl", "flstudio", "fl studio"]),
];

pub const STREAMING_SERVICE_ALIASES: &[(&str, &[&str])] = &[
    ("netflix", &["netflix", "netflx", "netfliks"]),
    ("youtube", &["youtube", "yt", "youtub", "jutube"]),
    ("prime", &["prime", "prime video", "amazon prime"]),
    ("disney", &["disney", "disney plus", "disneyplus"]),
    ("twitch", &["twitch", "twuicth", "twich"]),
    ("spotify", &["spotify", "spotfy"]),
];

pub const STREAMING_FOLLOWUP_CONFIRM: &[&str] = &[
    "yes",
    "yeah",
    "yep",
    "ja",
    "mach",
    "do it",
    "open it",
    "launch it",
    "play it",
    "open that",
    "launch that",
    "yes open it",
    "yes launch it",
    "yes play it",
];

pub const STREAMING_MORE_WORDS: &[&str] = &[
    "something else",
    "another one",
    "more like this",
    "more like that",
    "was anderes",
    "noch was",
    "gib mir was anderes",
];