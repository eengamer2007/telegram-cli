use anyhow;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::Mutex;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Widget},
    Terminal,
};

pub fn setup() -> anyhow::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    Ok(terminal)
}

pub fn exit(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

pub fn start_render_thread(
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    run_flag: std::sync::Arc<AtomicBool>,
) -> (
    tokio::sync::mpsc::Sender<RenderUpdate>,
    tokio::task::JoinHandle<()>,
) {
    let (mut state, render_tx) = TerminalState::new(terminal);

    // spawn a task to update the UI
    let render_handle = tokio::spawn(async move {
        while run_flag.load(Ordering::Acquire) {
            state.update();
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    });

    (render_tx, render_handle)
}

#[derive(PartialEq, Debug)]
pub enum RenderUpdate {
    NewMessage(tdlib::types::UpdateNewMessage),
    Exit,
}

pub struct TerminalState {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    rx: Receiver<RenderUpdate>,
}

impl TerminalState {
    pub fn new(terminal: Terminal<CrosstermBackend<io::Stdout>>) -> (Self, Sender<RenderUpdate>) {
        let (render_tx, render_rx) = mpsc::channel(5);
        (
            Self {
                terminal,
                rx: render_rx,
            },
            render_tx,
        )
    }

    pub fn update(&mut self) {
        if let Ok(update) = self.rx.try_recv() {
            match update {
                RenderUpdate::Exit => {
                    println!("exit the program");
                    exit(&mut self.terminal).unwrap();
                }
                _ => {}
            }
        }
    }
}
