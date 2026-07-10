use crate::pet::Pet;
use std::fs;
use std::path::PathBuf;

fn pet_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| String::from("."));
    PathBuf::from(home).join(".codepet.json")
}

pub fn save(pet: &Pet) {
    if let Ok(json) = serde_json::to_string_pretty(pet) {
        let _ = fs::write(pet_path(), json);
    }
}

pub fn load() -> Option<Pet> {
    let path = pet_path();
    if path.exists() {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
    } else {
        None
    }
}
