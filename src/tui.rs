use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
    Terminal,
};
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};

use crate::models::enums;
use crate::parse_json;
use crate::search;
use crate::command;

pub fn render_tui(key:String, proj_name: &str){
    enable_raw_mode().expect("can run in raw mode");
    
    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(200);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll works") {
                if let CEvent::Key(key) = event::read().expect("can read events") {
                    tx.send(enums::Event::Input(key)).expect("can send events");
                }
            }

            if last_tick.elapsed() >= tick_rate {
                if let Ok(_) = tx.send(enums::Event::Tick) {
                    last_tick = Instant::now();
                }
            }
        }
    });
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.clear().unwrap();
    let mut template_list_state = ListState::default();
    template_list_state.select(Some(0));

    loop {
        terminal.draw(|rect| {
            let size = rect.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints([Constraint::Percentage(80)].as_ref())
                .split(size);
            let template_list = render_templates(key.clone());
            rect.render_stateful_widget(template_list, chunks[0], &mut template_list_state);
        }).unwrap();

        match rx.recv().unwrap() {
            enums::Event::Input(event) => match event.code {
                KeyCode::Char('q') => {
                    println!("{:?}", template_list_state.selected());
                    disable_raw_mode().unwrap();
                    terminal.show_cursor().unwrap();
                    break;
                }
                KeyCode::Down => {
                    if let Some(selected) = template_list_state.selected() {
                        let templates_length = parse_json::read_json().expect("can fetch template list").len();
                        if selected >= templates_length - 1 {
                            template_list_state.select(Some(0));
                        } else {
                            template_list_state.select(Some(selected + 1));
                        }
                    }
                }
                KeyCode::Up => {
                    if let Some(selected) = template_list_state.selected() {
                        let templates_length = parse_json::read_json().expect("can fetch template list").len();
                        if selected > 0 {
                            template_list_state.select(Some(selected - 1));
                        } else {
                            template_list_state.select(Some(templates_length - 1));
                        }
                    }
                }
                KeyCode::Enter => {
                    if let Some(_) = template_list_state.selected() {
                        let templates = search::perform_search(parse_json::read_json().unwrap(), key.clone())
                            .expect("can fetch template list");
                        let selected_template = templates
                            .get(
                                template_list_state
                                    .selected()
                                    .expect("there is always a selected template"),
                            )
                            .unwrap();
                    command::git_clone(proj_name, selected_template.url.to_string(),&mut terminal);
                    break;
                    }
                }
                _ => {}
            },
            enums::Event::Tick => {}
        }
    }
}

fn render_templates<'a>(key: String) -> List<'a> {
    let pets = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title("Available Templates")
        .border_type(BorderType::Plain);

    let template_list = parse_json::read_json().expect("can fetch json");
    let filtered_template_list = search::perform_search(template_list, key).expect("can filter json");
    let items: Vec<_> = filtered_template_list
        .iter()
        .map(|submodule| {
            ListItem::new(Spans::from(vec![Span::styled(
                submodule.name.clone(),
                Style::default(),
            )]))
        })
        .collect();

    let list = List::new(items).block(pets).highlight_style(
        Style::default()
            .bg(Color::Yellow)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );

    list
}