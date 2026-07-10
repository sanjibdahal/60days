mod github;
mod pet;
mod storage;

fn main() {
    let pet = storage::load();

    let (mood, streak, last) = if let Some(p) = pet {
        let hours = p
            .last_commit
            .map(|lc| (chrono::Utc::now() - lc).num_hours())
            .unwrap_or(999);

        let last_str = match hours {
            0 => String::from("just now"),
            h if h < 24 => format!("{}h ago", h),
            h => format!("{}d ago", h / 24),
        };

        (p.mood_label().to_string(), p.streak, last_str)
    } else {
        (String::from("No pet"), 0, String::from("never"))
    };

    print!("{} | 🔥 {} days | ⏱ {}", mood, streak, last);
}
