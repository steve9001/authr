#[cfg(feature = "tui")]
use anyhow::Result;
#[cfg(feature = "tui")]
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
#[cfg(feature = "tui")]
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
#[cfg(feature = "tui")]
use std::{io, time::{Duration, SystemTime, UNIX_EPOCH}};
#[cfg(feature = "tui")]
use authr_core::model::Account;
#[cfg(feature = "tui")]
use authr_core::storage::load_accounts;
#[cfg(feature = "tui")]
use authr_core::totp;

#[cfg(feature = "tui")]
struct App {
    accounts: Vec<Account>,
    filter: String,
}

#[cfg(feature = "tui")]
impl App {
    fn new() -> Result<Self> {
        let accounts = load_accounts()?;
        Ok(Self {
            accounts,
            filter: String::new(),
        })
    }

    fn filtered_accounts(&self) -> Vec<&Account> {
        if self.filter.is_empty() {
            self.accounts.iter().collect()
        } else {
            self.accounts
                .iter()
                .filter(|a| a.name.to_lowercase().contains(&self.filter.to_lowercase()))
                .collect()
        }
    }
}

#[cfg(feature = "tui")]
pub fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app_result = App::new();
    let mut app = match app_result {
        Ok(app) => app,
        Err(e) => {
             disable_raw_mode()?;
             execute!(io::stdout(), LeaveAlternateScreen)?;
             return Err(e);
        }
    };

    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

#[cfg(feature = "tui")]
fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                 if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Esc => return Ok(()),
                        KeyCode::Char(c) => app.filter.push(c),
                        KeyCode::Backspace => { app.filter.pop(); },
                        _ => {}
                    }
                 }
            }
        }
    }
}

#[cfg(feature = "tui")]
fn ui(f: &mut ratatui::Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
        .split(f.area());

    // Calculate time remaining in 30s window (assuming 30s step)
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let remaining = 30 - (now % 30);
    
    // Accounts list
    let filtered = app.filtered_accounts();
    let items: Vec<ListItem> = filtered.iter().map(|acc| {
        let code = totp::generate_code(acc).unwrap_or_else(|_| "ERROR".to_string());
        
        // Color code based on remaining time
        let time_style = if remaining < 5 {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };

        let content = Line::from(vec![
            Span::styled(format!("{:<20}", acc.name), Style::default().fg(Color::Cyan)),
            Span::raw("   "),
            Span::styled(format!("{}", code), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw("   "),
            Span::styled(format!("({}s)", remaining), time_style),
        ]);
        ListItem::new(content)
    }).collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Accounts"));
    
    f.render_widget(list, chunks[0]);

    // Filter input
    let filter_text = format!("Filter: {}_", app.filter);
    let paragraph = Paragraph::new(filter_text)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).title("Search (ESC to quit)"));
    f.render_widget(paragraph, chunks[1]);
}

#[cfg(not(feature = "tui"))]
#[allow(dead_code)]
pub fn run() -> anyhow::Result<()> {
    anyhow::bail!("TUI feature not enabled");
}
