use std::error::Error;
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use std::process::Command;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::layout::{Constraint, Direction, Layout, Margin};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Paragraph, Wrap, Scrollbar, ScrollbarOrientation, ScrollbarState};
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
    last_command_output: Option<String>,
    input_mode: InputMode,
    output_scroll: usize,
    output_content_len: usize,
    output_view_height: usize,
    auto_scroll: bool,
    is_loading: bool,
    spinner_idx: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InputMode {
    Text,
    Command,
}

impl App {
    fn new() -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            last_submit: None,
            last_response: None,
            last_command_output: None,
            input_mode: InputMode::Text,
            output_scroll: 0,
            output_content_len: 0,
            output_view_height: 0,
            auto_scroll: false,
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
        match self.input_mode {
            InputMode::Text => {
                self.is_loading = true;
                self.last_response = None;
                self.auto_scroll = true;
                thread::spawn(move || {
                    let result = call_ollama(&prompt).map_err(|err| err.to_string());
                    let _ = tx.send(result);
                });
            }
            InputMode::Command => {
                self.last_command_output = Some(run_command(&prompt));
                self.auto_scroll = true;
            }
        }
        self.input.clear();
        self.cursor = 0;
    }

    fn scroll_up(&mut self, by: usize) {
        self.output_scroll = self.output_scroll.saturating_sub(by);
    }

    fn scroll_down(&mut self, by: usize) {
        let max_scroll = self
            .output_content_len
            .saturating_sub(self.output_view_height);
        self.output_scroll = (self.output_scroll + by).min(max_scroll);
    }

    fn scroll_to_start(&mut self) {
        self.output_scroll = 0;
    }

    fn scroll_to_end(&mut self) {
        let max_scroll = self
            .output_content_len
            .saturating_sub(self.output_view_height);
        self.output_scroll = max_scroll;
    }
}

#[derive(Serialize)]
struct OllamaRequest<'a> {
    model: &'a str,
    messages: Vec<OllamaMessage<'a>>,
    stream: bool,
}

#[derive(Serialize)]
struct OllamaMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct OllamaResponse {
    message: OllamaResponseMessage,
}

#[derive(Deserialize)]
struct OllamaResponseMessage {
    content: String,
}

fn call_ollama(prompt: &str) -> Result<String, Box<dyn Error>> {
    let client = reqwest::blocking::Client::new();
    let req = OllamaRequest {
        model: "qwen2.5-coder:14b",
        messages: vec![OllamaMessage {
            role: "user",
            content: prompt,
        }],
        stream: false,
    };
    let res: OllamaResponse = client
        .post("http://localhost:11434/api/chat")
        .json(&req)
        .send()?
        .json()?;
    Ok(res.message.content)
}

fn run_command(cmd: &str) -> String {
    // Run using the system shell so users can input typical command lines.
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output();

    match output {
        Ok(out) => {
            let mut text = String::new();
            if !out.stdout.is_empty() {
                text.push_str(String::from_utf8_lossy(&out.stdout).as_ref());
            }
            if !out.stderr.is_empty() {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(String::from_utf8_lossy(&out.stderr).as_ref());
            }
            if text.trim().is_empty() {
                "(command produced no output)".to_string()
            } else {
                text.trim_end().to_string()
            }
        }
        Err(err) => format!("Failed to run command: {}", err),
    }
}

fn inner_width(area: ratatui::layout::Rect) -> usize {
    area.width.saturating_sub(2) as usize
}

fn inner_height(area: ratatui::layout::Rect) -> usize {
    area.height.saturating_sub(2) as usize
}

fn truncate_input(input: &str, cursor: usize, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    let len = input.len();
    if len <= max_width {
        return input.to_string();
    }
    let cursor = cursor.min(len);
    let mut start = cursor.saturating_sub(max_width / 2);
    if start + max_width > len {
        start = len - max_width;
    }
    input[start..start + max_width].to_string()
}

fn line_count(text: &str) -> usize {
    let count = text.lines().count();
    if count == 0 { 1 } else { count }
}

fn cursor_x_in_view(input: &str, cursor: usize, max_width: usize) -> usize {
    if max_width == 0 {
        return 0;
    }
    let len = input.len();
    if len <= max_width {
        return cursor.min(len);
    }
    let cursor = cursor.min(len);
    let mut start = cursor.saturating_sub(max_width / 2);
    if start + max_width > len {
        start = len - max_width;
    }
    cursor.saturating_sub(start).min(max_width)
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
            app.auto_scroll = true;
        }
        if last_tick.elapsed() >= Duration::from_millis(100) {
            app.spinner_idx = (app.spinner_idx + 1) % spinner.len();
            last_tick = Instant::now();
        }

        terminal.draw(|frame| {
            let title_style = Style::default()
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD);
            let info_border = Style::default().fg(Color::Black).bg(Color::Gray);
            let input_border = Style::default().fg(Color::DarkGray);
            let help_border = Style::default().fg(Color::DarkGray);
            let info_text_style = Style::default().fg(Color::Blue).bg(Color::Gray);
            let help_text_style = Style::default().fg(Color::DarkGray);
            let input_text_style = Style::default().fg(Color::DarkGray);

            let area = frame.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(5), Constraint::Length(3), Constraint::Length(3)])
                .split(area);

            let info_title = match app.input_mode {
                InputMode::Text => {
                    if app.is_loading {
                        format!("Response {}", spinner[app.spinner_idx])
                    } else {
                        "Response".to_string()
                    }
                }
                InputMode::Command => "Command Output".to_string(),
            };
            let info_block = Block::bordered()
                .title(info_title)
                .title_style(title_style)
                .border_style(info_border);
            let raw_info_text = match app.input_mode {
                InputMode::Text => {
                    if app.is_loading {
                        "Loading...".to_string()
                    } else {
                        app.last_response
                            .as_deref()
                            .unwrap_or("Type your prompt below and press Enter.")
                            .to_string()
                    }
                }
                InputMode::Command => app
                    .last_command_output
                    .as_deref()
                    .unwrap_or("Type a command and press Enter.")
                    .to_string(),
            };
            let view_height = inner_height(chunks[0]);
            app.output_content_len = line_count(&raw_info_text);
            app.output_view_height = view_height;
            if app.auto_scroll {
                app.scroll_to_end();
                app.auto_scroll = false;
            } else if app.output_scroll
                > app
                    .output_content_len
                    .saturating_sub(app.output_view_height)
            {
                app.scroll_to_end();
            }

            let info = Paragraph::new(raw_info_text)
                .style(info_text_style)
                .scroll((app.output_scroll as u16, 0))
                .wrap(Wrap { trim: true })
                .block(info_block);
            frame.render_widget(info, chunks[0]);

            let mut scrollbar_state = ScrollbarState::new(app.output_content_len)
                .position(app.output_scroll);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .track_style(Style::default().fg(Color::DarkGray))
                .thumb_style(Style::default().fg(Color::Blue));
            frame.render_stateful_widget(
                scrollbar,
                chunks[0].inner(Margin { vertical: 1, horizontal: 0 }),
                &mut scrollbar_state,
            );

            let input_title = match app.input_mode {
                InputMode::Text => "Prompt (Text)",
                InputMode::Command => "Command (Direct)",
            };
            let input_block = Block::bordered()
                .title(input_title)
                .title_style(title_style)
                .border_style(input_border);
            let input_view = truncate_input(&app.input, app.cursor, inner_width(chunks[1]));
            let input = Paragraph::new(input_view)
                .style(input_text_style)
                .block(input_block)
                .wrap(Wrap { trim: false });
            frame.render_widget(input, chunks[1]);

            // Place cursor inside the input box, after the current character.
            let cursor_x = cursor_x_in_view(&app.input, app.cursor, inner_width(chunks[1]));
            let x = chunks[1].x + 1 + cursor_x as u16;
            let y = chunks[1].y + 1;
            frame.set_cursor_position((x, y));

            let help_block = Block::bordered()
                .title("Controls")
                .title_style(title_style)
                .border_style(help_border);
            let help_text = match app.input_mode {
                InputMode::Text => {
                    "Enter: Send to LLM | Tab: Toggle | Up/Down/PgUp/PgDn/Home/End: Scroll | Esc/Ctrl+C: Quit"
                }
                InputMode::Command => {
                    "Enter: Run command | Tab: Toggle | Up/Down/PgUp/PgDn/Home/End: Scroll | Esc/Ctrl+C: Quit"
                }
            };
            let help = Paragraph::new(help_text)
                .style(help_text_style)
                .wrap(Wrap { trim: true })
                .block(help_block);
            frame.render_widget(help, chunks[2]);
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
                    KeyCode::Up => app.scroll_up(1),
                    KeyCode::Down => app.scroll_down(1),
                    KeyCode::PageUp => app.scroll_up(app.output_view_height.max(1)),
                    KeyCode::PageDown => app.scroll_down(app.output_view_height.max(1)),
                    KeyCode::Home => app.scroll_to_start(),
                    KeyCode::End => app.scroll_to_end(),
                    KeyCode::Tab => {
                        app.input_mode = match app.input_mode {
                            InputMode::Text => InputMode::Command,
                            InputMode::Command => InputMode::Text,
                        };
                        app.input.clear();
                        app.cursor = 0;
                        app.auto_scroll = true;
                    }
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
