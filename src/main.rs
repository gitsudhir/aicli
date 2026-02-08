use std::error::Error;
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Block, Paragraph, Wrap};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use serde::{Deserialize, Serialize};

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

struct App {
    input: String,
    cursor: usize,
    last_submit: Option<String>,
    last_response: Option<String>,
    is_loading: bool,
    spinner_idx: usize,
}

impl App {
    fn new() -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            last_submit: None,
            last_response: None,
            is_loading: false,
            spinner_idx: 0,
        }
    }

    fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor, c);
        self.cursor += 1;
    }

    fn delete_char(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor -= 1;
        self.input.remove(self.cursor);
    }

    fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    fn move_right(&mut self) {
        if self.cursor < self.input.len() {
            self.cursor += 1;
        }
    }

    fn submit(&mut self, tx: mpsc::Sender<Result<String, String>>) {
        if self.input.trim().is_empty() {
            return;
        }
        if self.is_loading {
            return;
        }
        let prompt = self.input.clone();
        self.last_submit = Some(prompt.clone());
        self.is_loading = true;
        self.last_response = None;
        thread::spawn(move || {
            let result = call_ollama(&prompt).map_err(|err| err.to_string());
            let _ = tx.send(result);
        });
        self.input.clear();
        self.cursor = 0;
    }
}

#[derive(Serialize)]
struct OllamaRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

fn call_ollama(prompt: &str) -> Result<String, Box<dyn Error>> {
    let client = reqwest::blocking::Client::new();
    let req = OllamaRequest {
        model: "llama3",
        prompt,
        stream: false,
    };
    let res: OllamaResponse = client
        .post("http://localhost:11434/api/generate")
        .json(&req)
        .send()?
        .json()?;
    Ok(res.response)
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    let (tx, rx) = mpsc::channel::<Result<String, String>>();
    let mut last_tick = Instant::now();
    let spinner = ["|", "/", "-", "\\"];
    loop {
        if let Ok(result) = rx.try_recv() {
            app.is_loading = false;
            match result {
                Ok(resp) => app.last_response = Some(resp),
                Err(err) => app.last_response = Some(format!("Error: {}", err)),
            }
        }
        if last_tick.elapsed() >= Duration::from_millis(100) {
            app.spinner_idx = (app.spinner_idx + 1) % spinner.len();
            last_tick = Instant::now();
        }

        terminal.draw(|frame| {
            let area = frame.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Length(3)])
                .split(area);

            let info_title = if app.is_loading {
                format!("Llama3 Response {}", spinner[app.spinner_idx])
            } else {
                "Llama3 Response".to_string()
            };
            let info_block = Block::bordered().title(info_title);
            let info_text = if app.is_loading {
                "Loading...".to_string()
            } else {
                app.last_response
                    .as_deref()
                    .unwrap_or("Type your prompt below and press Enter.")
                    .to_string()
            };
            let info = Paragraph::new(info_text).wrap(Wrap { trim: true }).block(info_block);
            frame.render_widget(info, chunks[0]);

            let input_block = Block::bordered().title("Prompt");
            let input = Paragraph::new(app.input.as_str())
                .block(input_block)
                .wrap(Wrap { trim: false });
            frame.render_widget(input, chunks[1]);

            // Place cursor inside the input box, after the current character.
            let x = chunks[1].x + 1 + app.cursor as u16;
            let y = chunks[1].y + 1;
            frame.set_cursor(x, y);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(());
                    }
                    KeyCode::Esc => return Ok(()),
                    KeyCode::Enter => app.submit(tx.clone()),
                    KeyCode::Left => app.move_left(),
                    KeyCode::Right => app.move_right(),
                    KeyCode::Backspace => app.delete_char(),
                    KeyCode::Char(ch) => app.insert_char(ch),
                    _ => {}
                }
            }
        }
    }
}
