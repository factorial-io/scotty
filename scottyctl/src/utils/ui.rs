use super::status_line::Status;
use super::status_line::StatusLine;
use std::io::IsTerminal;
use std::sync::Arc;
use std::sync::RwLock;

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

pub struct Ui {
    status_line: Option<Arc<RwLock<StatusLine>>>,
}

impl Ui {
    pub fn new() -> Self {
        let stdout = std::io::stdout();

        let is_terminal = stdout.is_terminal();
        let status_line = if is_terminal {
            // Only create and animate the status line if we have a terminal
            let status_line_inner = Arc::new(RwLock::new(StatusLine::new()));

            // Initialize the status line text before starting the animation
            {
                let mut sl = status_line_inner.write().unwrap();
                sl.set_status(Status::Running, "Initializing...");
            }

            // Start the animation thread
            {
                let mut rt = Some(StatusLine::start_render(status_line_inner.clone()));
                let mut at = Some(StatusLine::start_animation(status_line_inner.clone()));
                let mut sl = status_line_inner.write().unwrap();
                sl.animation_thread = at.take();
                sl.render_thread = rt.take();
            }

            Some(status_line_inner)
        } else {
            None
        };

        Ui { status_line }
    }

    pub fn set_status(&self, status_text: &str, status: Status) {
        if status != Status::Running {
            self.eprintln(format!("{} {}", status.get_emoji(), status_text).as_str());
        }
        if let Some(status_line) = &self.status_line {
            if let Ok(mut sl) = status_line.write() {
                sl.set_status(status, status_text);
            }
        }
    }

    pub fn new_status_line(&self, message: impl AsRef<str>) {
        self.set_status(message.as_ref(), Status::Running)
    }

    pub fn failed(&self, message: impl AsRef<str>) {
        self.set_status(message.as_ref(), Status::Failed)
    }

    pub fn success(&self, message: impl AsRef<str>) {
        self.set_status(message.as_ref(), Status::Succeeded)
    }

    pub fn println(&self, msg: impl AsRef<str>) {
        if let Some(status_line) = &self.status_line {
            status_line.read().unwrap().println(msg.as_ref());
        } else {
            println!("{}", msg.as_ref());
        }
    }

    pub fn eprintln(&self, msg: impl AsRef<str>) {
        if let Some(status_line) = &self.status_line {
            status_line.read().unwrap().eprintln(msg.as_ref());
        } else {
            eprintln!("{}", msg.as_ref());
        }
    }

    /// Check if output is going to a terminal (as opposed to being piped or redirected)
    pub fn is_terminal(&self) -> bool {
        self.status_line.is_some()
    }

    pub async fn run<F>(&self, x: F) -> anyhow::Result<()>
    where
        F: AsyncFn,
    {
        match x.call().await {
            Ok(result) => {
                if let Some(sl) = &self.status_line {
                    sl.read().unwrap().clear_line();
                }
                if !result.is_empty() {
                    println!("\n{result}");
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
