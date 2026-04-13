use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::io;

use super::blacklist::{apply_selection, scan_loaded_modules, BlacklistEntry};

#[derive(PartialEq)]
enum Mode {
    Select,
    Help,
}

struct App {
    entries: Vec<BlacklistEntry>,
    selected: Vec<bool>,
    state: ListState,
    mode: Mode,
    category_filter: Option<String>,
}

impl App {
    fn new(entries: Vec<BlacklistEntry>) -> Self {
        let selected = entries.iter().map(|e| e.currently_blacklisted).collect();
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            entries,
            selected,
            state,
            mode: Mode::Select,
            category_filter: None,
        }
    }

    fn filtered_indices(&self) -> Vec<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                self.category_filter
                    .as_ref()
                    .map_or(true, |cat| &e.category == cat)
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn current_index(&self) -> Option<usize> {
        let filtered = self.filtered_indices();
        let list_pos = self.state.selected()?;
        filtered.into_iter().nth(list_pos)
    }

    fn toggle(&mut self) {
        if let Some(idx) = self.current_index() {
            self.selected[idx] = !self.selected[idx];
        }
    }

    fn select_all(&mut self) {
        for i in &mut self.selected {
            *i = true;
        }
    }

    fn deselect_all(&mut self) {
        for i in &mut self.selected {
            *i = false;
        }
    }

    fn apply_recommended(&mut self) {
        for (i, entry) in self.entries.iter().enumerate() {
            self.selected[i] = entry.recommended;
        }
    }

    fn set_category(&mut self, cat: Option<&str>) {
        self.category_filter = cat.map(String::from);
        self.state.select(Some(0));
    }

    fn categories(&self) -> Vec<String> {
        let mut cats: Vec<String> = self.entries.iter().map(|e| e.category.clone()).collect();
        cats.sort();
        cats.dedup();
        cats
    }

    fn count_selected(&self) -> usize {
        self.selected.iter().filter(|&&s| s).count()
    }
}

pub fn run_tui() -> Result<Vec<String>> {
    let entries = scan_loaded_modules()?;

    if entries.is_empty() {
        println!("로드된 커널 모듈이 없습니다.");
        return Ok(vec![]);
    }

    let app = App::new(entries);
    let result = run_app(app)?;

    if let Some(chosen) = result {
        if !chosen.is_empty() {
            apply_selection(&chosen)?;
        }
        Ok(chosen)
    } else {
        Ok(vec![])
    }
}

fn run_app(mut app: App) -> Result<Option<Vec<String>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut _should_apply = false;

    loop {
        terminal.draw(|f| render(f, &app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match app.mode {
                    Mode::Select => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            disable_raw_mode()?;
                            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                            return Ok(None);
                        }
                        KeyCode::Char('d') | KeyCode::Enter => {
                            _should_apply = true;
                            break;
                        }
                        KeyCode::Char(' ') => app.toggle(),
                        KeyCode::Char('a') => app.select_all(),
                        KeyCode::Char('n') => app.deselect_all(),
                        KeyCode::Char('r') => app.apply_recommended(),
                        KeyCode::Char('h') => app.mode = Mode::Help,
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.state.select(app.state.selected().and_then(|s| {
                                if s > 0 {
                                    Some(s - 1)
                                } else {
                                    None
                                }
                            }))
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let max = app.filtered_indices().len().saturating_sub(1);
                            app.state.select(app.state.selected().and_then(|s| {
                                if s < max {
                                    Some(s + 1)
                                } else {
                                    None
                                }
                            }));
                        }
                        KeyCode::Char('1') => app.set_category(None),
                        KeyCode::Char(c) if ('2'..='9').contains(&c) => {
                            let cats = app.categories();
                            let idx = (c as usize) - ('2' as usize);
                            if idx < cats.len() {
                                app.set_category(Some(&cats[idx]));
                            }
                        }
                        _ => {}
                    },
                    Mode::Help => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('h') => {
                            app.mode = Mode::Select;
                        }
                        _ => {}
                    },
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    if _should_apply {
        let chosen: Vec<String> = app
            .entries
            .iter()
            .enumerate()
            .filter(|(i, _)| app.selected[*i])
            .map(|(_, e)| e.module.clone())
            .collect();
        Ok(Some(chosen))
    } else {
        Ok(None)
    }
}

fn render(f: &mut Frame, app: &App) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(size);

    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " stream-cli ",
            Style::default().fg(Color::Black).bg(Color::Cyan).bold(),
        ),
        Span::raw("  커널 모듈 블랙리스트 관리"),
    ]))
    .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(title, chunks[0]);

    let filtered = app.filtered_indices();
    let items: Vec<ListItem> = filtered
        .iter()
        .map(|&idx| {
            let entry = &app.entries[idx];
            let sel = app.selected[idx];

            let check = if sel { "■" } else { "□" };
            let rec = if entry.recommended { "★" } else { " " };
            let locked = if entry.currently_blacklisted {
                "🔒"
            } else {
                " "
            };

            let (check_color, name_color) = if sel {
                (Color::Green, Color::White)
            } else {
                (Color::DarkGray, Color::DarkGray)
            };

            let warning = match entry.category.as_str() {
                "GPU/DRM" | "HID" | "SD" | "Ethernet" => " ⚠",
                _ => "",
            };

            let line = Line::from(vec![
                Span::styled(
                    format!(" {} ", check),
                    Style::default().fg(check_color).bold(),
                ),
                Span::styled(rec.to_string(), Style::default().fg(Color::Yellow)),
                Span::raw(locked),
                Span::styled(
                    format!(" {:<22}", entry.module),
                    Style::default().fg(name_color),
                ),
                Span::styled(
                    format!("[{:<8}]", entry.category),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    format!("{:>6}K", entry.size / 1024),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("  {}{}", entry.reason, warning),
                    if warning.is_empty() {
                        Style::default().fg(Color::Gray)
                    } else {
                        Style::default().fg(Color::Red)
                    },
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::NONE))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, chunks[1], &mut app.state.clone());

    let help_text = if app.mode == Mode::Help {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(" ↑/k", Style::default().fg(Color::Yellow)),
                Span::raw(" 위   "),
                Span::styled("↓/j", Style::default().fg(Color::Yellow)),
                Span::raw(" 아래   "),
                Span::styled("Space", Style::default().fg(Color::Yellow)),
                Span::raw(" 선택   "),
                Span::styled("a", Style::default().fg(Color::Yellow)),
                Span::raw(" 전체   "),
                Span::styled("n", Style::default().fg(Color::Yellow)),
                Span::raw(" 해제   "),
                Span::styled("r", Style::default().fg(Color::Yellow)),
                Span::raw(" 권장   "),
            ]),
            Line::from(vec![
                Span::styled(" d/Enter", Style::default().fg(Color::Green)),
                Span::raw(" 적용   "),
                Span::styled("q/Esc", Style::default().fg(Color::Red)),
                Span::raw(" 종료   "),
                Span::styled(" 1", Style::default().fg(Color::Yellow)),
                Span::raw(" 전체 카테고리   "),
                Span::styled("2-9", Style::default().fg(Color::Yellow)),
                Span::raw(" 카테고리 필터"),
            ]),
        ]
    } else {
        let cat_label = app.category_filter.as_deref().unwrap_or("전체");
        vec![Line::from(vec![
            Span::styled(
                format!(" 선택: {}  ", app.count_selected()),
                Style::default().fg(Color::Green).bold(),
            ),
            Span::styled(
                format!("[{}] ", cat_label),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw("Space:선택  a:전체  n:해제  r:권장  d:적용  h:도움말  q:종료"),
        ])]
    };

    let footer = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::TOP))
        .style(Style::default().fg(Color::White));
    f.render_widget(footer, chunks[2]);

    if app.mode == Mode::Help {
        let help_area = centered_rect(50, 12, size);
        f.render_widget(Clear, help_area);
        let help_block = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  단축키 도움말",
                Style::default().fg(Color::Cyan).bold(),
            )),
            Line::from(""),
            Line::from("  ↑/k, ↓/j     커서 이동"),
            Line::from("  Space         선택/해제 토글"),
            Line::from("  a             전체 선택"),
            Line::from("  n             전체 해제"),
            Line::from("  r             권장 항목 자동 선택"),
            Line::from("  d / Enter     변경 적용"),
            Line::from("  q / Esc       종료 (미적용)"),
            Line::from("  1~9           카테고리 필터"),
            Line::from(""),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" 도움말 (h/q: 닫기) "),
        )
        .style(Style::default().fg(Color::White));
        f.render_widget(help_block, help_area);
    }
}

fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_width = std::cmp::min(r.width * percent_x / 100, 60);
    let x = (r.width.saturating_sub(popup_width)) / 2;
    let y = (r.height.saturating_sub(height)) / 2;
    Rect::new(x, y, popup_width, height)
}
