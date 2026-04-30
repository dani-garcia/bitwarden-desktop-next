//! Print session events to stdout. Run, then lock/unlock/suspend the system.
//!
//! ```
//! cargo run -p session-events --example print
//! ```

use std::pin::pin;

use futures_util::StreamExt;
use tokio::signal;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "session_events=debug,info".into()),
        )
        .init();

    println!("Listening for session events. Lock the screen, sleep the machine, etc.");
    println!("Press Ctrl-C to exit.\n");

    let mut stream = pin!(session_events::event_stream());

    loop {
        tokio::select! {
            ev = stream.next() => match ev {
                Some(ev) => println!("{ev:?}"),
                None => {
                    eprintln!("Stream ended; exiting.");
                    break;
                }
            },
            _ = signal::ctrl_c() => {
                println!("\nCtrl-C received; exiting.");
                break;
            }
        }
    }
}
