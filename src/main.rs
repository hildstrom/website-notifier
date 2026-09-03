//! Website Notifier is a small utility that sends a notification when websites
//! go down or come up. The notification provides:
//! * DOWN or UP
//! * the URL
//! * the date and time for the event
//!
//! This Rust utility uses Tokio for learning and demonstration purposes.
//! This could easily be rewritten in conventional sync Rust or in a script.
//!
//! Before sending a DOWN notification, a connectivity probe is sent to a
//! reliable third-party host. If that probe also fails, the site failure is
//! attributed to a local connectivity problem and the notification is suppressed.

/// Reliable third-party endpoint used to verify internet connectivity.
/// Cloudflare's 1.1.1.1 is used because it is fast, stable, and independent
/// of the sites being monitored.
const CONNECTIVITY_PROBE_URL: &str = "https://1.1.1.1";

use std::env;
use chrono::{DateTime, Local};
use reqwest::Client;

/// the main function
#[tokio::main]
async fn main() {
    println!("Website Notifier");

    // read CLI args
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <ntfy.sh url> <url to check> [...]", args[0]);
        return;
    }
    let ntfyurl = args[1].clone();
    println!("ntfyurl: {}", ntfyurl);
    let urls: Vec<String> = args[2..].to_vec();
    println!("urls to check: {:?}", urls);

    // need to start one monitoring task per url to check
    let mut tasks = Vec::new();
    for url in urls {
        let ntfyurl = ntfyurl.clone();
        let url = url.clone();
        tasks.push(tokio::spawn(async move {
            monitor_url(ntfyurl, url).await;
        }));
    }

    // wait tasks to complete, but these run forever
    for task in tasks {
        task.await.unwrap();
    }
}

/// monitor the url and send a notification when needed
async fn monitor_url(ntfyurl: String, url: String) {
    let mut web_state = true;
    println!("Starting monitor for URL: {}", url);
    loop {
        let client = Client::new();
        let res = client.get(&url).send().await;

        match res {
            Ok(_r) => {
                println!("UP {}", url);
                if !web_state {
                    send_notification(&ntfyurl, "UP ".to_string() + &url).await;
                    web_state = true;
                }
            },
            Err(e) => {
                eprintln!("DOWN {} {}", url, e);
                if web_state {
                    if internet_is_reachable(&client).await {
                        send_notification(&ntfyurl, "DOWN ".to_string() + &url).await;
                        web_state = false;
                    } else {
                        eprintln!("Connectivity probe failed — suppressing DOWN notification for {}", url);
                    }
                }
            }
        };

        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
    }
}

/// check whether the internet is reachable by probing a reliable third-party host
async fn internet_is_reachable(client: &Client) -> bool {
    match client.get(CONNECTIVITY_PROBE_URL).send().await {
        Ok(_) => true,
        Err(e) => {
            eprintln!("Connectivity probe to {} failed: {}", CONNECTIVITY_PROBE_URL, e);
            false
        }
    }
}

/// send the notification and
/// retry once per minute if there is a problem, like the network not being up yet
async fn send_notification(ntfyurl:&String, notification: String) {
    let client = Client::new();
    let current: DateTime<Local> = Local::now();
    let current_fmt = current.format(" %Y-%m-%d %H:%M:%S").to_string();
    let notification = notification + &current_fmt;
    println!("{}", notification);
    loop {
        let res = client.post(ntfyurl)
            .body(notification.clone())
            .send()
            .await;
        match res {
            Ok(_r) => {
                println!("Notification {} Sent!", notification);
                break;
            },
            Err(e) => {
                eprintln!("Notification {} Error, {}, retrying after 60s", notification, e);
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            }
        };
    }
}

