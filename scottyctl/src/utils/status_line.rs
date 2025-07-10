use crossterm::{
    cursor::MoveToColumn,
    execute,
    style::Print,
    terminal::{Clear, ClearType},
};
use std::io::stdout;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

#[derive(PartialEq)]
pub enum Status {
    Running,
    Succeeded,
    Failed,
}
impl Status {
    fn new() -> Self {
        Status::Running
    }
    pub fn get_emoji(&self) -> &str {
        match self {
            Status::Running => "💭",
            Status::Succeeded => "🚀",
            Status::Failed => "💥",
        }
    }
}

pub struct StatusLine {
    text: String,
    throbber_index: usize,
    throbber_chars: Vec<char>,
    status: Status,
    is_running: Arc<AtomicBool>,
    pub animation_thread: Option<thread::JoinHandle<()>>,
    pub render_thread: Option<thread::JoinHandle<()>>,
    is_dirty: Arc<AtomicBool>,
}

impl StatusLine {
    pub fn new() -> Self {
        StatusLine {
            text: String::new(),
            throbber_index: 0,
            throbber_chars: vec!['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'],
            is_running: Arc::new(AtomicBool::new(false)),
            animation_thread: None,
            render_thread: None,
            is_dirty: Arc::new(AtomicBool::new(false)),
            status: Status::new(),
        }
    }

    fn trigger_render(&self) {
        self.is_dirty.store(true, Ordering::SeqCst);
    }

    pub fn set_status(&mut self, status: Status, status_text: &str) {
        self.status = status;
        self.text = status_text.to_string();
        self.trigger_render();
    }

    fn advance_throbber(&mut self) {
        self.throbber_index = (self.throbber_index + 1) % self.throbber_chars.len();
        self.trigger_render();
    }

    fn draw(&self) -> String {
        if self.text.is_empty() {
            return String::new();
        }

        let throbber = self.throbber_chars[self.throbber_index];
        format!("{} {} {}", throbber, self.status.get_emoji(), self.text)
    }

    fn render(&self) {
        let status = self.draw();
        execute!(stdout(), MoveToColumn(0), Print(status)).unwrap_or_else(|_| {
            // Handle error gracefully instead of panic
        });
        self.is_dirty.store(false, Ordering::SeqCst);
    }

    pub fn clear_line(&self) {
        execute!(stdout(), MoveToColumn(0), Clear(ClearType::CurrentLine)).unwrap_or_else(|_| {
            // Handle error gracefully instead of panic
        });
    }

    pub fn println(&self, msg: &str) {
        self.clear_line();
        println!("{msg}");
        self.trigger_render();
    }

    pub fn eprintln(&self, msg: &str) {
        self.clear_line();
        eprintln!("{msg}");
        self.trigger_render();
    }

    pub fn start_render(status_line: Arc<RwLock<StatusLine>>) -> thread::JoinHandle<()> {
        let (is_dirty, is_running) = {
            let sl = status_line.read().unwrap();
            sl.is_running.store(true, Ordering::SeqCst);
            (sl.is_dirty.clone(), sl.is_running.clone())
        };

        let status_line = status_line.clone();

        thread::spawn(move || {
            while is_running.load(Ordering::SeqCst) {
                if is_dirty.load(Ordering::SeqCst) {
                    status_line.read().unwrap().render();
                }
                thread::sleep(Duration::from_millis(1));
            }
        })
    }

    pub fn start_animation(status_line: Arc<RwLock<Self>>) -> thread::JoinHandle<()> {
        let is_running = {
            let sl = status_line.read().unwrap();
            sl.is_running.store(true, Ordering::SeqCst);
            sl.is_running.clone()
        };

        let status_line = status_line.clone();

        thread::spawn(move || {
            while is_running.load(Ordering::SeqCst) {
                if let Ok(mut status) = status_line.write() {
                    status.advance_throbber();
                }

                thread::sleep(Duration::from_millis(75));
            }
        })
    }

    pub fn stop_animation(&mut self) {
        let handles = vec![self.animation_thread.take(), self.render_thread.take()];
        self.is_running.store(false, Ordering::SeqCst);
        for handle in handles.into_iter().flatten() {
            if handle.thread().id() != thread::current().id() {
                handle
                    .join()
                    .unwrap_or_else(|_| eprintln!("Failed to join thread"));
            }
        }
    }
}

impl Drop for StatusLine {
    fn drop(&mut self) {
        self.stop_animation();
    }
}
