use std::collections::HashMap;
use zbus::Connection;
use zbus::zvariant::OwnedValue;

fn format_progress(position_us: i64, length_us: i64, width: usize) -> String {
    if length_us == 0 {
        return "─".repeat(width);
    }
    let ratio = (position_us as f64 / length_us as f64).clamp(0.0, 1.0);
    let filled = (ratio * width as f64).round() as usize;
    format!("{}{}", "▓".repeat(filled), "░".repeat(width - filled))
}

fn format_time(us: i64) -> String {
    let secs = us / 1_000_000;
    format!("{}:{:02}", secs / 60, secs % 60)
}

#[tokio::main]
async fn main() -> zbus::Result<()> {
    let connection = Connection::session().await?;

    // Check for control flags first
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let player_bus = "org.mpris.MediaPlayer2.playerctld";
        let proxy = zbus::Proxy::new(
            &connection,
            player_bus,
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
        )
        .await?;

        match args[1].as_str() {
            "--play-pause" => {
                proxy.call_method("PlayPause", &()).await?;
            }
            "--next" => {
                proxy.call_method("Next", &()).await?;
            }
            "--prev" => {
                proxy.call_method("Previous", &()).await?;
            }
            _ => eprintln!("Unknown flag: {}", args[1]),
        }
        return Ok(());
    }

    let player_bus = "org.mpris.MediaPlayer2.playerctld";

    let proxy = zbus::Proxy::new(
        &connection,
        player_bus,
        "/org/mpris/MediaPlayer2",
        "org.freedesktop.DBus.Properties",
    )
    .await?;

    // Get returns a Variant wrapping the actual value — unwrap it first
    let metadata_variant: OwnedValue = proxy
        .call("Get", &("org.mpris.MediaPlayer2.Player", "Metadata"))
        .await?;

    let metadata: HashMap<String, OwnedValue> = metadata_variant.try_into()?;

    // for (key, value) in &metadata {
    //    println!("{}: {:?}", key, value);
    // }

    let title: String = metadata
        .get("xesam:title")
        .and_then(|v| v.downcast_ref::<String>().ok())
        .unwrap_or_else(|| "Unknown Title".to_string());

    let artist: String = metadata
        .get("xesam:artist")
        .and_then(|v| v.downcast_ref::<zbus::zvariant::Array>().ok())
        .and_then(|arr| arr.get::<String>(0).ok().flatten())
        .unwrap_or_else(|| "Unknown Artist".to_string());

    // Track length (microseconds) from Metadata
    let length_us: i64 = metadata
        .get("mpris:length")
        .and_then(|v| v.downcast_ref::<u64>().ok())
        .map(|v| v as i64)
        .unwrap_or(0);

    // Current position (microseconds) — separate property, not in Metadata
    let position_variant: OwnedValue = proxy
        .call("Get", &("org.mpris.MediaPlayer2.Player", "Position"))
        .await?;
    let position_us: i64 = position_variant.try_into().unwrap_or(0);

    let status_variant: OwnedValue = proxy
        .call("Get", &("org.mpris.MediaPlayer2.Player", "PlaybackStatus"))
        .await?;
    let status: String = status_variant.try_into()?;

    let icon = match status.as_str() {
        "Playing" => "▶",
        "Paused" => "⏸",
        _ => "⏹",
    };

    let bar = format_progress(position_us, length_us, 10);
    let pos_str = format_time(position_us);
    let len_str = format_time(length_us);

    // Truncate long titles for status bar
    let max_len = 30;
    let display_title = if title.chars().count() > max_len {
        let truncated: String = title.chars().take(max_len).collect();
        format!("{}…", truncated)
    } else {
        title.clone()
    };

    // Determine color based on player source (via track URL/ID as a proxy)
    let color = if title.is_empty() {
        "colour245" // grey fallback
    } else if metadata
        .get("xesam:url")
        .and_then(|v| v.downcast_ref::<String>().ok())
        .map(|u| u.contains("spotify"))
        .unwrap_or(false)
    {
        "colour46" // Spotify green
    } else {
        "colour196" // red (YT Music / other)
    };

    println!(
        "#[fg={}]{} {} — {} [{}] {}/{}#[fg=default]",
        color, icon, display_title, artist, bar, pos_str, len_str
    );
    Ok(())
}
