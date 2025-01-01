use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use tokio::{signal, sync::Notify};
use tracing::info;

#[derive(Clone, Debug)]
pub struct StopFlag {
    flag: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl StopFlag {
    pub fn new() -> Self {
        StopFlag {
            flag: Arc::new(AtomicBool::new(false)),
            notify: Arc::new(Notify::new()),
        }
    }

    pub fn stop(&self) {
        self.flag.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    pub fn is_stopped(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    pub async fn wait(&self) {
        self.notify.notified().await;
    }
}

pub fn register_signal_handler(stop_flag: &StopFlag) {
    {
        let stop_flag = stop_flag.clone();
        tokio::spawn(async move {
            let _ = signal::ctrl_c().await;
            info!("Ctrl-C received, initiating graceful shutdown...");
            stop_flag.stop();
        });
    }
    {
        let stop_flag = stop_flag.clone();

        tokio::spawn(async move {
            let mut terminate = signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install SIGTERM handler");
            let mut interrupt = signal::unix::signal(signal::unix::SignalKind::interrupt())
                .expect("failed to install SIGINT handler");
            let mut hangup = signal::unix::signal(signal::unix::SignalKind::hangup())
                .expect("failed to install SIGHUP handler");

            tokio::select! {
                _ = terminate.recv() => {
                    info!("SIGTERM received, initiating graceful shutdown...");
                }
                _ = interrupt.recv() => {
                    info!("SIGINT received, initiating graceful shutdown...");
                }
                _ = hangup.recv() => {
                    info!("SIGHUP received, initiating graceful shutdown...");
                }
            }

            stop_flag.stop();
        });
    }
}
