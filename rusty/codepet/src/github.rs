use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct GithubEvent {
    #[serde(rename = "type")]
    event_type: String,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct GithubRepo {
    name: String,
    fork: bool,
}

pub struct GithubStats {
    pub commits_today: u32,
    pub total_commits: u32,
    pub streak: u32,
    pub last_commit: Option<DateTime<Utc>>,
    pub top_language: String,
}

pub fn fetch_stats(username: &str) -> Result<GithubStats, String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("codepet/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let mut req = client.get(format!(
        "https://api.github.com/users/{}/events?per_page=100",
        username
    ));
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        req = req.header("Authorization", format!("Bearer {}", token));
    }

    let events: Vec<GithubEvent> = req
        .send()
        .map_err(|e| e.to_string())?
        .json()
        .map_err(|e| e.to_string())?;

    let push_events: Vec<&GithubEvent> = events
        .iter()
        .filter(|e| e.event_type == "PushEvent")
        .collect();

    let now = Utc::now();
    let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
    let commits_today = push_events
        .iter()
        .filter(|e| e.created_at >= today_start)
        .count() as u32;

    let last_commit = push_events.first().map(|e| e.created_at);

    let mut streak = 0u32;
    let mut check_date = now.date_naive();
    loop {
        let day_start = check_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let day_end = check_date.and_hms_opt(23, 59, 59).unwrap().and_utc();
        let has_commit = push_events
            .iter()
            .any(|e| e.created_at >= day_start && e.created_at <= day_end);
        if has_commit {
            streak += 1;
            check_date = check_date.pred_opt().unwrap();
        } else {
            break;
        }
        if streak > 365 {
            break;
        }
    }

    let top_language =
        fetch_top_language(&client, username).unwrap_or_else(|_| String::from("Unknown"));

    Ok(GithubStats {
        commits_today,
        total_commits: push_events.len() as u32,
        streak,
        last_commit,
        top_language,
    })
}

fn fetch_top_language(
    client: &reqwest::blocking::Client,
    username: &str,
) -> Result<String, String> {
    let mut req = client.get(format!(
        "https://api.github.com/users/{}/repos?per_page=100&sort=pushed",
        username
    ));
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        req = req.header("Authorization", format!("Bearer {}", token));
    }
    let repos: Vec<GithubRepo> = req
        .send()
        .map_err(|e| e.to_string())?
        .json()
        .map_err(|e| e.to_string())?;

    let mut lang_counts: HashMap<String, u64> = HashMap::new();

    for repo in repos.iter().filter(|r| !r.fork).take(10) {
        if let Ok(langs) = client
            .get(format!(
                "https://api.github.com/repos/{}/{}/languages",
                username, repo.name
            ))
            .send()
            .and_then(|r| r.json::<HashMap<String, u64>>())
        {
            for (lang, bytes) in langs {
                *lang_counts.entry(lang).or_insert(0) += bytes;
            }
        }
    }

    Ok(lang_counts
        .into_iter()
        .max_by_key(|(_, v)| *v)
        .map(|(k, _)| k)
        .unwrap_or_else(|| String::from("Unknown")))
}
