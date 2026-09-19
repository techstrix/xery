use std::io;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use xery_lib::Vault;

const INK: Color = Color::Rgb(10, 17, 29);
const STEEL: Color = Color::Rgb(121, 148, 171);
const ICE: Color = Color::Rgb(220, 232, 240);
const BRASS: Color = Color::Rgb(218, 167, 79);

#[derive(Clone, Copy)]
enum FormKind {
    Create,
    Login,
    Add,
    Read,
}

struct Form {
    kind: FormKind,
    fields: Vec<String>,
    focus: usize,
}

enum Screen {
    Vaults,
    Vault,
    Form(Form),
    Reveal { identifier: String, secret: String },
}

struct App {
    vault: Vault,
    vaults: Vec<String>,
    selected: usize,
    active: Option<String>,
    screen: Screen,
    status: String,
    quit: bool,
}

impl App {
    fn new() -> Self {
        let mut app = Self {
            vault: Vault::default(),
            vaults: Vec::new(),
            selected: 0,
            active: None,
            screen: Screen::Vaults,
            status: String::new(),
            quit: false,
        };
        app.refresh_vaults();
        app
    }

    fn refresh_vaults(&mut self) {
        match self.vault.list_vaults() {
            Ok(vaults) => {
                self.vaults = vaults;
                self.selected = self.selected.min(self.vaults.len().saturating_sub(1));
            }
            Err(error) => self.status = error.to_string(),
        }
    }

    fn open_form(&mut self, kind: FormKind, fields: usize) {
        self.status.clear();
        self.screen = Screen::Form(Form {
            kind,
            fields: vec![String::new(); fields],
            focus: 0,
        });
    }

    fn handle_key(&mut self, code: KeyCode) {
        match &mut self.screen {
            Screen::Vaults => match code {
                KeyCode::Char('q') => self.quit = true,
                KeyCode::Char('n') => self.open_form(FormKind::Create, 3),
                KeyCode::Up | KeyCode::Char('k') => {
                    self.selected = self.selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.selected + 1 < self.vaults.len() {
                        self.selected += 1;
                    }
                }
                KeyCode::Enter if !self.vaults.is_empty() => {
                    self.open_form(FormKind::Login, 1);
                }
                _ => {}
            },
            Screen::Vault => match code {
                KeyCode::Char('a') => self.open_form(FormKind::Add, 4),
                KeyCode::Char('r') => self.open_form(FormKind::Read, 2),
                KeyCode::Char('l') => self.logout(),
                _ => {}
            },
            Screen::Reveal { .. } => {
                if matches!(code, KeyCode::Enter | KeyCode::Esc) {
                    self.screen = Screen::Vault;
                    self.status = "Password hidden".into();
                }
            }
            Screen::Form(form) => match code {
                KeyCode::Esc => {
                    self.status.clear();
                    self.screen = if self.active.is_some() {
                        Screen::Vault
                    } else {
                        Screen::Vaults
                    };
                }
                KeyCode::Tab | KeyCode::Down => form.focus = (form.focus + 1) % form.fields.len(),
                KeyCode::BackTab | KeyCode::Up => {
                    form.focus = (form.focus + form.fields.len() - 1) % form.fields.len();
                }
                KeyCode::Backspace => {
                    form.fields[form.focus].pop();
                }
                KeyCode::Enter => self.submit_form(),
                KeyCode::Char(character) => form.fields[form.focus].push(character),
                _ => {}
            },
        }
    }

    fn submit_form(&mut self) {
        let Screen::Form(form) = &self.screen else {
            return;
        };
        let kind = form.kind;
        let fields = form.fields.clone();
        if fields.iter().any(String::is_empty) {
            self.status = "Complete every field".into();
            return;
        }

        match kind {
            FormKind::Create if fields[1] != fields[2] => {
                self.status = "Vault passwords do not match".into();
            }
            FormKind::Create => match self
                .vault
                .create_vault(fields[0].clone(), fields[1].clone())
            {
                Ok(()) => {
                    self.refresh_vaults();
                    self.screen = Screen::Vaults;
                    self.status = "Vault created".into();
                }
                Err(error) => self.status = error.to_string(),
            },
            FormKind::Login => {
                let name = self.vaults[self.selected].clone();
                match self.vault.login_vault(&name, &fields[0]) {
                    Ok(()) => {
                        self.active = Some(name);
                        self.screen = Screen::Vault;
                        self.status = "Vault unlocked".into();
                    }
                    Err(error) => self.status = error.to_string(),
                }
            }
            FormKind::Add => {
                match self
                    .vault
                    .add_password(&fields[0], &fields[1], &fields[2], &fields[3])
                {
                    Ok(()) => {
                        self.screen = Screen::Vault;
                        self.status = "Password saved".into();
                    }
                    Err(error) => self.status = error.to_string(),
                }
            }
            FormKind::Read => match self.vault.read_password(&fields[1], &fields[0]) {
                Ok(secret) if !secret.is_empty() => {
                    self.screen = Screen::Reveal {
                        identifier: fields[0].clone(),
                        secret,
                    };
                    self.status.clear();
                }
                Ok(_) => self.status = "No password found for that identifier".into(),
                Err(error) => self.status = error.to_string(),
            },
        }
    }

    fn logout(&mut self) {
        match self.vault.logout_vault() {
            Ok(()) => {
                self.active = None;
                self.screen = Screen::Vaults;
                self.status = "Vault locked".into();
                self.refresh_vaults();
            }
            Err(error) => self.status = error.to_string(),
        }
    }
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let result = run(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut app = App::new();
    while !app.quit {
        terminal.draw(|frame| draw(frame, &app))?;
        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.handle_key(key.code);
        }
    }
    Ok(())
}

fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    frame.render_widget(Block::default().style(Style::default().bg(INK)), area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .margin(1)
        .split(area);

    let title = match &app.active {
        Some(vault) => format!("XERY  /  {vault}"),
        None => "XERY  /  LOCKED".into(),
    };
    frame.render_widget(
        Paragraph::new(title)
            .style(Style::default().fg(BRASS).add_modifier(Modifier::BOLD))
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(STEEL),
            ),
        rows[0],
    );

    match &app.screen {
        Screen::Vaults => draw_vaults(frame, rows[1], app),
        Screen::Vault => draw_vault(frame, rows[1]),
        Screen::Form(form) => draw_form(frame, rows[1], form),
        Screen::Reveal { identifier, secret } => {
            let text = vec![
                Line::from(Span::styled(identifier, Style::default().fg(STEEL))),
                Line::from(""),
                Line::from(Span::styled(
                    secret,
                    Style::default().fg(ICE).add_modifier(Modifier::BOLD),
                )),
            ];
            frame.render_widget(panel(Paragraph::new(text), " Revealed password "), rows[1]);
        }
    }

    let help = match app.screen {
        Screen::Vaults => "↑/↓ select  Enter unlock  n new  q quit",
        Screen::Vault => "a add  r reveal  l lock",
        Screen::Form(_) => "Tab next  Enter confirm  Esc cancel",
        Screen::Reveal { .. } => "Enter/Esc hide",
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(&app.status, Style::default().fg(BRASS)),
            Span::raw(if app.status.is_empty() { "" } else { "  ·  " }),
            Span::styled(help, Style::default().fg(STEEL)),
        ]))
        .alignment(Alignment::Center),
        rows[2],
    );
}

fn draw_vaults(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = if app.vaults.is_empty() {
        vec![ListItem::new("No vaults yet — press n to create one")]
    } else {
        app.vaults
            .iter()
            .map(String::as_str)
            .map(ListItem::new)
            .collect()
    };
    let mut state =
        ListState::default().with_selected((!app.vaults.is_empty()).then_some(app.selected));
    let list = List::new(items)
        .block(
            Block::default()
                .title(" Vaults ")
                .borders(Borders::ALL)
                .border_style(STEEL),
        )
        .style(Style::default().fg(ICE))
        .highlight_symbol("  ◆  ")
        .highlight_style(Style::default().fg(BRASS).add_modifier(Modifier::BOLD));
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_vault(frame: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(Span::styled(
            "Vault unlocked",
            Style::default().fg(ICE).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Add a credential or reveal one by its identifier."),
        Line::from(Span::styled(
            "Your vault password is required for either action.",
            Style::default().fg(STEEL),
        )),
    ];
    frame.render_widget(panel(Paragraph::new(text), " Secure session "), area);
}

fn draw_form(frame: &mut Frame, area: Rect, form: &Form) {
    let (title, labels, masked): (&str, &[&str], &[usize]) = match form.kind {
        FormKind::Create => (
            " New vault ",
            &["Name", "Vault password", "Confirm password"],
            &[1, 2],
        ),
        FormKind::Login => (" Unlock vault ", &["Vault password"], &[0]),
        FormKind::Add => (
            " Add password ",
            &["Identifier", "Username", "Password", "Vault password"],
            &[2, 3],
        ),
        FormKind::Read => (" Reveal password ", &["Identifier", "Vault password"], &[1]),
    };
    let lines = form.fields.iter().enumerate().flat_map(|(index, value)| {
        let value = if masked.contains(&index) {
            "•".repeat(value.chars().count())
        } else {
            value.clone()
        };
        let value = if value.is_empty() {
            " ".to_string()
        } else {
            value
        };
        let marker = if index == form.focus { "◆" } else { " " };
        [
            Line::from(Span::styled(labels[index], Style::default().fg(STEEL))),
            Line::from(vec![
                Span::styled(format!("{marker} "), Style::default().fg(BRASS)),
                Span::styled(value, Style::default().fg(ICE).add_modifier(Modifier::BOLD)),
            ]),
        ]
    });
    frame.render_widget(
        panel(Paragraph::new(lines.collect::<Vec<_>>()), title),
        area,
    );
}

fn panel<'a>(content: Paragraph<'a>, title: &'a str) -> Paragraph<'a> {
    content
        .style(Style::default().fg(ICE))
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(STEEL),
        )
        .alignment(Alignment::Left)
}
