#[derive(Debug)]
pub enum Status {
    Running,
    Failed,
    Success,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Running => write!(f, "Running:"),
            Status::Failed => write!(f, "Failed:"),
            Status::Success => write!(f, "Success:"),
        }
    }
}

pub trait AsyncFn {
    type Future: std::future::Future<Output = anyhow::Result<String>>;
    fn call(self) -> Self::Future;
}

impl<F, Fut> AsyncFn for F
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<String>>,
{
    type Future = Fut;
    fn call(self) -> Self::Future {
        self()
    }
}

use std::fmt::Debug;
use std::sync::Arc;

use std::sync::RwLock;

struct StatusLineInner {
    message: String,
    status: Status,
}

impl StatusLineInner {
    fn new(message: impl AsRef<str>) -> Self {
        let s = Self {
            message: message.as_ref().into(),
            status: Status::Running,
        };
        s.print();
        s
    }

    fn print(&self) {
        eprintln!("{} {}", self.status, self.message);
    }

    fn update(&mut self, message: impl AsRef<str>, status: Status) {
        self.message = message.as_ref().into();
        self.status = status;
        self.print();
    }
}

impl Debug for StatusLineInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatusLineInner")
            .field("message", &self.message)
            .field("status", &self.status)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct StatusLine {
    inner: Arc<RwLock<StatusLineInner>>,
}

impl StatusLine {
    pub fn new(message: impl AsRef<str>) -> Self {
        let inner = StatusLineInner::new(message);
        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    fn set_status(&self, message: impl AsRef<str>, status: Status) {
        let mut inner = self.inner.write().unwrap();
        inner.update(message, status);
    }

    pub fn new_status_line(&self, message: impl AsRef<str>) {
        self.set_status(message, Status::Running)
    }

    pub fn failed(&self, message: impl AsRef<str>) {
        self.set_status(message, Status::Failed)
    }

    pub fn success(&self, message: impl AsRef<str>) {
        self.set_status(message, Status::Success)
    }

    pub async fn run<F>(&self, x: F) -> anyhow::Result<()>
    where
        F: AsyncFn,
    {
        match x.call().await {
            Ok(result) => {
                if !result.is_empty() {
                    println!("{}", result);
                }

                Ok(())
            }
            Err(e) => {
                self.failed(e.to_string());
                Err(e)
            }
        }
    }
}
