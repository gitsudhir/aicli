use std::io;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use futures::StreamExt;
use rag::{Config as RagConfig, answer_query};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Margin};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let rag_cfg = Arc::new(RagConfig::from_env());
    let mut app = App::new(rag_cfg);
    let res = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

struct App {
    input: String,
    cursor: usize,
    last_submit: Option<String>,
    last_command_output: Option<String>,
    rag_context: Option<String>,
    rag_answer: Option<String>,
    rag_cfg: Arc<RagConfig>,
    input_mode: InputMode,
    output_focus: OutputFocus,
    context_scroll: usize,
    context_content_len: usize,
    context_view_height: usize,
    context_auto_scroll: bool,
    answer_scroll: usize,
    answer_content_len: usize,
    answer_view_height: usize,
    answer_auto_scroll: bool,
    is_loading: bool,
    spinner_idx: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InputMode {
    Text,
    Command,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputFocus {
    Context,
    Answer,
}

enum Response {
    Rag(Result<(String, String), String>),
    Index(Result<(), String>),
    Command(String),
}

impl App {
    fn new(rag_cfg: Arc<RagConfig>) -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            last_submit: None,
            last_command_output: None,
            rag_context: None,
            rag_answer: None,
            rag_cfg,
            input_mode: InputMode::Text,
            output_focus: OutputFocus::Answer,
            context_scroll: 0,
            context_content_len: 0,
            context_view_height: 0,
            context_auto_scroll: false,
            answer_scroll: 0,
            answer_content_len: 0,
            answer_view_height: 0,
            answer_auto_scroll: false,
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

    fn submit(&mut self, tx: mpsc::UnboundedSender<Response>) {
        if self.input.trim().is_empty() || self.is_loading {
            return;
        }

        let prompt = self.input.clone();
        self.last_submit = Some(prompt.clone());

        match self.input_mode {
            InputMode::Text => {
                self.is_loading = true;
                self.answer_auto_scroll = true;
                self.context_auto_scroll = true;
                self.rag_context = None;
                self.rag_answer = None;
                let rag_cfg = self.rag_cfg.clone();
                tokio::task::spawn_blocking(move || {
                    let result = answer_query(&rag_cfg, &prompt).map_err(|err| err.to_string());
                    let _ = tx.send(Response::Rag(result));
                });
            }
            InputMode::Command => {
                self.is_loading = true;
                self.answer_auto_scroll = true;
                tokio::task::spawn_blocking(move || {
                    let _ = tx.send(Response::Command(run_command(&prompt)));
                });
            }
        }

        self.input.clear();
        self.cursor = 0;
    }

    fn index_now(&mut self, tx: mpsc::UnboundedSender<Response>) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        self.context_auto_scroll = true;
        self.answer_auto_scroll = true;
        self.rag_context = Some("Indexing...".to_string());
        self.rag_answer = Some("Building embeddings and updating Qdrant.".to_string());
        let rag_cfg = self.rag_cfg.clone();
        tokio::task::spawn_blocking(move || {
            let result = rag::index_corpus(&rag_cfg, None).map_err(|err| err.to_string());
            let _ = tx.send(Response::Index(result));
        });
    }

    fn scroll_up(&mut self, by: usize) {
        match self.output_focus {
            OutputFocus::Context => {
                self.context_scroll = self.context_scroll.saturating_sub(by);
            }
            OutputFocus::Answer => {
                self.answer_scroll = self.answer_scroll.saturating_sub(by);
            }
        }
    }

    fn scroll_down(&mut self, by: usize) {
        match self.output_focus {
            OutputFocus::Context => {
                let max_scroll = self
                    .context_content_len
                    .saturating_sub(self.context_view_height);
                self.context_scroll = (self.context_scroll + by).min(max_scroll);
            }
            OutputFocus::Answer => {
                let max_scroll = self
                    .answer_content_len
                    .saturating_sub(self.answer_view_height);
                self.answer_scroll = (self.answer_scroll + by).min(max_scroll);
            }
        }
    }

    fn scroll_to_start(&mut self) {
        match self.output_focus {
            OutputFocus::Context => self.context_scroll = 0,
            OutputFocus::Answer => self.answer_scroll = 0,
        }
    }

    fn scroll_to_end(&mut self) {
        match self.output_focus {
            OutputFocus::Context => {
                self.context_scroll = self
                    .context_content_len
                    .saturating_sub(self.context_view_height);
            }
            OutputFocus::Answer => {
                self.answer_scroll = self
                    .answer_content_len
                    .saturating_sub(self.answer_view_height);
            }
        }
    }

    fn focused_view_height(&self) -> usize {
        match self.output_focus {
            OutputFocus::Context => self.context_view_height,
            OutputFocus::Answer => self.answer_view_height,
        }
    }
}

fn run_command(cmd: &str) -> String {
    let output = Command::new("sh").arg("-c").arg(cmd).output();

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

fn draw_ui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    let spinner = ["|", "/", "-", "\\"];

    terminal.draw(|frame| {
        let title_style = Style::default().fg(Color::Black).add_modifier(Modifier::BOLD);
        let info_border = Style::default().fg(Color::Black);
        let input_border = Style::default().fg(Color::DarkGray);
        let help_border = Style::default().fg(Color::DarkGray);
        let info_text_style = Style::default().fg(Color::Blue);
        let help_text_style = Style::default().fg(Color::DarkGray);
        let input_text_style = Style::default().fg(Color::DarkGray);

        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(8),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(area);
        let output_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(chunks[0]);

        let (context_text, answer_text) = match app.input_mode {
            InputMode::Text => (
                app.rag_context
                    .as_deref()
                    .unwrap_or("Context will appear here after you run a query.")
                    .to_string(),
                if app.is_loading {
                    "Loading...".to_string()
                } else {
                    app.rag_answer
                        .as_deref()
                        .unwrap_or("Type your prompt below and press Enter.")
                        .to_string()
                },
            ),
            InputMode::Command => (
                "Context is available in Text mode.".to_string(),
                if app.is_loading {
                    "Running command...".to_string()
                } else {
                    app.last_command_output
                        .as_deref()
                        .unwrap_or("Type a command and press Enter.")
                        .to_string()
                },
            ),
        };

        let context_title = match app.output_focus {
            OutputFocus::Context => "Context *",
            OutputFocus::Answer => "Context",
        };

        let answer_title = match app.input_mode {
            InputMode::Text => {
                if app.is_loading {
                    format!(
                        "Answer {}{}",
                        spinner[app.spinner_idx],
                        match app.output_focus {
                            OutputFocus::Answer => " *",
                            _ => "",
                        }
                    )
                } else if app.output_focus == OutputFocus::Answer {
                    "Answer *".to_string()
                } else {
                    "Answer".to_string()
                }
            }
            InputMode::Command => {
                if app.is_loading {
                    format!(
                        "Command Output {}{}",
                        spinner[app.spinner_idx],
                        match app.output_focus {
                            OutputFocus::Answer => " *",
                            _ => "",
                        }
                    )
                } else if app.output_focus == OutputFocus::Answer {
                    "Command Output *".to_string()
                } else {
                    "Command Output".to_string()
                }
            }
        };

        let context_block = Block::bordered()
            .title(context_title)
            .title_style(title_style)
            .border_style(info_border);
        let answer_block = Block::bordered()
            .title(answer_title)
            .title_style(title_style)
            .border_style(info_border);

        let context_view_height = inner_height(output_chunks[0]);
        app.context_content_len = line_count(&context_text);
        app.context_view_height = context_view_height;
        if app.context_auto_scroll {
            app.context_scroll = app.context_content_len.saturating_sub(app.context_view_height);
            app.context_auto_scroll = false;
        } else if app.context_scroll > app.context_content_len.saturating_sub(app.context_view_height) {
            app.context_scroll = app.context_content_len.saturating_sub(app.context_view_height);
        }

        let answer_view_height = inner_height(output_chunks[1]);
        app.answer_content_len = line_count(&answer_text);
        app.answer_view_height = answer_view_height;
        if app.answer_auto_scroll {
            app.answer_scroll = app.answer_content_len.saturating_sub(app.answer_view_height);
            app.answer_auto_scroll = false;
        } else if app.answer_scroll > app.answer_content_len.saturating_sub(app.answer_view_height) {
            app.answer_scroll = app.answer_content_len.saturating_sub(app.answer_view_height);
        }

        let context = Paragraph::new(context_text)
            .style(info_text_style)
            .scroll((app.context_scroll as u16, 0))
            .wrap(Wrap { trim: true })
            .block(context_block);
        frame.render_widget(context, output_chunks[0]);

        let mut context_scrollbar = ScrollbarState::new(app.context_content_len).position(app.context_scroll);
        let context_scrollbar_widget = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .track_style(Style::default().fg(Color::DarkGray))
            .thumb_style(Style::default().fg(Color::Blue));
        frame.render_stateful_widget(
            context_scrollbar_widget,
            output_chunks[0].inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut context_scrollbar,
        );

        let answer = Paragraph::new(answer_text)
            .style(info_text_style)
            .scroll((app.answer_scroll as u16, 0))
            .wrap(Wrap { trim: true })
            .block(answer_block);
        frame.render_widget(answer, output_chunks[1]);

        let mut answer_scrollbar = ScrollbarState::new(app.answer_content_len).position(app.answer_scroll);
        let answer_scrollbar_widget = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .track_style(Style::default().fg(Color::DarkGray))
            .thumb_style(Style::default().fg(Color::Blue));
        frame.render_stateful_widget(
            answer_scrollbar_widget,
            output_chunks[1].inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut answer_scrollbar,
        );

        let input_title = match app.input_mode {
            InputMode::Text => "Prompt (RAG)  [Ctrl+R: Index]",
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
                "Enter: Run RAG | F2/Ctrl+R: Index | Tab: Mode | Ctrl+O: Focus | Up/Down/PgUp/PgDn/Home/End: Scroll | Esc/Ctrl+C: Quit"
            }
            InputMode::Command => {
                "Enter: Run command | F2/Ctrl+R: Index | Tab: Mode | Ctrl+O: Focus | Up/Down/PgUp/PgDn/Home/End: Scroll | Esc/Ctrl+C: Quit"
            }
        };
        let help = Paragraph::new(help_text)
            .style(help_text_style)
            .wrap(Wrap { trim: true })
            .block(help_block);
        frame.render_widget(help, chunks[2]);
    })?;

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<Response>();
    let mut events = EventStream::new();
    let mut spinner_tick = tokio::time::interval(Duration::from_millis(100));
    spinner_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    draw_ui(terminal, app)?;

    loop {
        tokio::select! {
            _ = spinner_tick.tick() => {
                if app.is_loading {
                    app.spinner_idx = (app.spinner_idx + 1) % 4;
                    draw_ui(terminal, app)?;
                }
            }
            maybe_result = rx.recv() => {
                if let Some(result) = maybe_result {
                    app.is_loading = false;
                    match result {
                        Response::Rag(res) => match res {
                            Ok((ctx, ans)) => {
                                app.rag_context = Some(ctx);
                                app.rag_answer = Some(ans);
                            }
                            Err(err) => {
                                app.rag_context = Some(String::new());
                                app.rag_answer = Some(format!("Error: {}", err));
                            }
                        },
                        Response::Index(res) => match res {
                            Ok(()) => {
                                app.rag_context = Some("Indexing complete.".to_string());
                                app.rag_answer = Some("You can now run a RAG query.".to_string());
                            }
                            Err(err) => {
                                app.rag_context = Some("Indexing failed.".to_string());
                                app.rag_answer = Some(format!("Error: {}", err));
                            }
                        },
                        Response::Command(output) => {
                            app.last_command_output = Some(output);
                        }
                    }
                    app.context_auto_scroll = true;
                    app.answer_auto_scroll = true;
                    draw_ui(terminal, app)?;
                }
            }
            maybe_event = events.next() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                        match key.code {
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(()),
                            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => app.index_now(tx.clone()),
                            KeyCode::F(2) => app.index_now(tx.clone()),
                            KeyCode::Esc => return Ok(()),
                            KeyCode::Enter => app.submit(tx.clone()),
                            KeyCode::Up => app.scroll_up(1),
                            KeyCode::Down => app.scroll_down(1),
                            KeyCode::PageUp => app.scroll_up(app.focused_view_height().max(1)),
                            KeyCode::PageDown => app.scroll_down(app.focused_view_height().max(1)),
                            KeyCode::Home => app.scroll_to_start(),
                            KeyCode::End => app.scroll_to_end(),
                            KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                app.output_focus = match app.output_focus {
                                    OutputFocus::Context => OutputFocus::Answer,
                                    OutputFocus::Answer => OutputFocus::Context,
                                };
                            }
                            KeyCode::Tab => {
                                app.input_mode = match app.input_mode {
                                    InputMode::Text => InputMode::Command,
                                    InputMode::Command => InputMode::Text,
                                };
                                app.input.clear();
                                app.cursor = 0;
                                app.context_auto_scroll = true;
                                app.answer_auto_scroll = true;
                            }
                            KeyCode::Left => app.move_left(),
                            KeyCode::Right => app.move_right(),
                            KeyCode::Backspace => app.delete_char(),
                            KeyCode::Char(ch) => app.insert_char(ch),
                            _ => {}
                        }
                        draw_ui(terminal, app)?;
                    }
                    Some(Ok(_)) => {}
                    Some(Err(_)) => {}
                    None => return Ok(()),
                }
            }
        }
    }
}
