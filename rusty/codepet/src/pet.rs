use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Mood {
    Excited,
    Happy,
    Hungry,
    Sleeping,
    Dead,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pet {
    pub name: String,
    pub username: String,
    pub streak: u32,
    pub last_commit: Option<DateTime<Utc>>,
    pub commits_today: u32,
    pub total_commits: u32,
    pub top_language: String,
    pub mood: Mood,
    pub x: f64,
    pub y: f64,
    pub dx: f64,
    pub dy: f64,
    pub frame: usize,
    pub last_updated: DateTime<Utc>,
}

impl Pet {
    pub fn new(name: String, username: String) -> Self {
        Pet {
            name,
            username,
            streak: 0,
            last_commit: None,
            commits_today: 0,
            total_commits: 0,
            top_language: String::from("Unknown"),
            mood: Mood::Happy,
            x: 5.0,
            y: 5.0,
            dx: 0.5,
            dy: 0.3,
            frame: 0,
            last_updated: Utc::now(),
        }
    }

    pub fn update_mood(&mut self) {
        let hours = self
            .last_commit
            .map(|lc| (Utc::now() - lc).num_hours())
            .unwrap_or(999);

        self.mood = match hours {
            h if h < 24 && self.streak >= 3 => Mood::Excited,
            h if h < 24 => Mood::Happy,
            h if h < 48 => Mood::Hungry,
            h if h < 120 => Mood::Sleeping,
            _ => Mood::Dead,
        };
    }

    pub fn tick(&mut self, width: f64, height: f64) {
        if self.mood == Mood::Dead {
            return;
        }

        let speed = match self.mood {
            Mood::Excited => 1.2,
            Mood::Happy => 0.8,
            Mood::Hungry => 0.4,
            Mood::Sleeping => 0.1,
            Mood::Dead => 0.0,
        };

        self.x += self.dx * speed;
        self.y += self.dy * speed;

        let max_x = width - 10.0;
        let max_y = height - 8.0;

        if self.x <= 1.0 || self.x >= max_x {
            self.dx = -self.dx;
            self.x = self.x.clamp(1.0, max_x);
        }
        if self.y <= 1.0 || self.y >= max_y {
            self.dy = -self.dy;
            self.y = self.y.clamp(1.0, max_y);
        }

        self.frame = self.frame.wrapping_add(1);
    }

    pub fn ascii_art(&self) -> Vec<&'static str> {
        let f = self.frame / 8 % 2;
        match self.mood {
            Mood::Excited => match f {
                0 => vec!["  /\\_/\\  ", " ( ^w^ ) ", "  >*^*<  ", " ~ ~ ~ ~ "],
                _ => vec!["  /\\_/\\  ", " ( ^w^ ) ", "  >*^*<  ", "~ ~ ~ ~  "],
            },
            Mood::Happy => match f {
                0 => vec!["  /\\_/\\  ", " ( ^.^ ) ", "  > ^ <  ", "  ~   ~  "],
                _ => vec!["  /\\_/\\  ", " ( ^.^ ) ", "  > ^ <  ", " ~   ~   "],
            },
            Mood::Hungry => match f {
                0 => vec!["  /\\_/\\  ", " ( -.- ) ", "  > ~ <  ", "  . . .  "],
                _ => vec!["  /\\_/\\  ", " ( -.- ) ", "  > ~ <  ", " . . .   "],
            },
            Mood::Sleeping => vec!["  /\\_/\\  ", " ( z.z ) ", "  > - <  ", "  z Z z  "],
            Mood::Dead => vec!["  /\\_/\\  ", " ( x.x ) ", "  > - <  ", "  R . I  "],
        }
    }

    pub fn mood_label(&self) -> &'static str {
        match self.mood {
            Mood::Excited => "Excited 🔥",
            Mood::Happy => "Happy 😸",
            Mood::Hungry => "Hungry 😿",
            Mood::Sleeping => "Sleeping 😴",
            Mood::Dead => "Dead 💀",
        }
    }

    pub fn mood_color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self.mood {
            Mood::Excited => Color::Yellow,
            Mood::Happy => Color::Green,
            Mood::Hungry => Color::Red,
            Mood::Sleeping => Color::Blue,
            Mood::Dead => Color::DarkGray,
        }
    }

    pub fn status_message(&self) -> &'static str {
        let f = self.frame / 20 % 3;
        match self.mood {
            Mood::Excited => ["On fire!", "Keep going!", "Unstoppable!"][f],
            Mood::Happy => ["Purring...", "Content~", "Life is good"][f],
            Mood::Hungry => ["Feed me commits...", "Please code...", "Missing you..."][f],
            Mood::Sleeping => ["Zzzzz...", "z Z z...", "Dreaming of code..."][f],
            Mood::Dead => ["You abandoned me...", "Gone forever...", "RIP"][f],
        }
    }
}
