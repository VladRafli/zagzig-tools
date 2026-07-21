use std::time::{Duration, Instant};

pub fn format_ago(instant: Instant) -> String {
    let secs = instant.elapsed().as_secs();
    if secs < 1 {
        "just now".to_string()
    } else if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86_400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86_400)
    }
}

pub fn format_ms(duration: Duration) -> String {
    format!("{} ms", duration.as_millis())
}

#[derive(Default, Clone)]
pub struct TextInput {
    pub value: String,
}

impl TextInput {
    pub fn push(&mut self, c: char) {
        self.value.push(c);
    }

    pub fn backspace(&mut self) {
        self.value.pop();
    }

    pub fn clear(&mut self) {
        self.value.clear();
    }
}
