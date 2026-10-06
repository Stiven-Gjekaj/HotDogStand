//! The application: it holds the store and the windows, and connects them.
//!
//! The windows show what the Rust code gives them. Each change goes to the
//! store first. Then the windows read the store again.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};

use hds_core::export::{write_csv, write_json};
use hds_core::query::{AssigneeFilter, Column, StatusFilter};
use hds_core::{Edit, Label, NewTicket, Person, Priority, Query, Status, Ticket, Title};
use hds_store::Store;
use jiff::tz::TimeZone;
use slint::{Color, ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::format::{self, Names};
use crate::frame;
use crate::ui::{
    AboutWindow, AppMenu, HistoryEntry, LabelChip, LabelsWindow, MainWindow, Markdown,
    PeopleWindow, PersonRow, Theme, ThemeName, TicketRow, TicketWindow,
};

/// The column of the list that each header sorts by. The labels column does
/// not sort.
const COLUMNS: [Option<Column>; 7] = [
    Some(Column::Id),
    Some(Column::Title),
    Some(Column::Status),
    Some(Column::Priority),
    Some(Column::Assignee),
    None,
    Some(Column::Updated),
];

/// The filters of the "Show" list, in its order.
const STATUS_FILTERS: [StatusFilter; 5] = [
    StatusFilter::NotClosed,
    StatusFilter::All,
    StatusFilter::Is(Status::Open),
    StatusFilter::Is(Status::InProgress),
    StatusFilter::Is(Status::Closed),
];

/// The colors of the Labels window, in its order.
const LABEL_COLORS: [&str; 8] = [
    "#c42b1c", "#c46a00", "#b8a000", "#2e8b57", "#1f7a8c", "#3a6ea5", "#7a3ab0", "#6b6b6b",
];

fn model<T: Clone + 'static>(items: Vec<T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(items))
}

fn strings<I: IntoIterator<Item = S>, S: Into<SharedString>>(items: I) -> ModelRc<SharedString> {
    model(items.into_iter().map(Into::into).collect())
}

fn color(hex: &str) -> Color {
    let (r, g, b) = format::hex_color(hex);
    Color::from_rgb_u8(r, g, b)
}

fn index_of<T: PartialEq>(items: &[T], item: &T) -> i32 {
    items.iter().position(|i| i == item).map_or(0, |p| p as i32)
}

/// The data that the windows show, as the store gave it.
#[derive(Default)]
struct Data {
    tickets: Vec<Ticket>,
    people: Vec<Person>,
    labels: Vec<Label>,
}

impl Data {
    fn person_names(&self) -> HashMap<i64, String> {
        self.people.iter().map(|p| (p.id, p.name.clone())).collect()
    }

    fn label_names(&self) -> HashMap<i64, String> {
        self.labels.iter().map(|l| (l.id, l.name.clone())).collect()
    }
}

/// A ticket window and the ids behind the items of its lists.
struct TicketView {
    window: TicketWindow,
    /// The ticket, or `None` for a ticket that does not exist yet.
    id: Option<i64>,
    /// The person of each item of the "Assigned to" list.
    assignees: Vec<Option<i64>>,
    /// The label of each check box.
    labels: Vec<i64>,
}

pub struct App {
    store: RefCell<Store>,
    path: PathBuf,
    zone: TimeZone,
    main: MainWindow,
    data: RefCell<Data>,
    query: RefCell<Query>,
    /// The positions in `data.tickets` of the rows of the list, in order.
    shown: RefCell<Vec<usize>>,
    assignee_filters: RefCell<Vec<AssigneeFilter>>,
    label_filters: RefCell<Vec<Option<i64>>>,
    /// The open ticket windows. A ticket that does not exist yet has a
    /// negative key.
    ticket_windows: RefCell<HashMap<i64, TicketView>>,
    next_new_key: Cell<i64>,
    /// The step of the next ticket window down and to the right of the main
    /// window, so that two windows do not open at one place.
    cascade: Cell<u8>,
    people_window: RefCell<Option<PeopleWindow>>,
    labels_window: RefCell<Option<LabelsWindow>>,
    about_window: RefCell<Option<AboutWindow>>,
    theme: Cell<ThemeName>,
    dark: Cell<bool>,
    system_frame: Cell<bool>,
}

/// Gives a window the theme of the application. Each window has its own copy
/// of the `Theme` global, so each one needs this.
macro_rules! style {
    ($app:expr, $window:expr) => {{
        let theme = $window.global::<Theme<'_>>();
        theme.set_name($app.theme.get());
        theme.set_dark($app.dark.get());
        let menu = $window.global::<AppMenu<'_>>();
        menu.set_native(cfg!(target_os = "macos"));
        menu.set_system_frame($app.system_frame.get());
        menu.set_mark_aero($app.theme.get() == ThemeName::Aero);
        menu.set_mark_hot_dog_stand($app.theme.get() == ThemeName::HotDogStand);
        menu.set_mark_dark($app.dark.get());
        menu.set_mark_system_frame($app.system_frame.get());
    }};
}

/// Connects the menus of the macOS menu bar of a window to the application.
macro_rules! wire_menu {
    ($app:expr, $window:expr) => {{
        let menu = $window.global::<AppMenu<'_>>();
        let weak = Rc::downgrade($app);
        let call = move |f: fn(&Rc<App>)| {
            let weak = weak.clone();
            move || {
                if let Some(app) = weak.upgrade() {
                    f(&app);
                }
            }
        };
        menu.on_new_ticket(call(|app| app.open_ticket(None)));
        menu.on_export_json(call(|app| app.export_json()));
        menu.on_export_csv(call(|app| app.export_csv()));
        menu.on_show_people(call(|app| app.show_people()));
        menu.on_show_labels(call(|app| app.show_labels()));
        menu.on_show_about(call(|app| app.show_about()));
        let weak = Rc::downgrade($app);
        menu.on_set_theme(move |theme| {
            if let Some(app) = weak.upgrade() {
                app.set_theme(theme);
            }
        });
        let weak = Rc::downgrade($app);
        menu.on_set_dark(move |on| {
            if let Some(app) = weak.upgrade() {
                app.set_dark(on);
            }
        });
        let weak = Rc::downgrade($app);
        menu.on_set_system_frame(move |on| {
            if let Some(app) = weak.upgrade() {
                app.set_system_frame(on);
            }
        });
    }};
}

/// Wires the frame of a window to the frame actions, for each kind of window.
macro_rules! wire_frame {
    ($window:expr) => {{
        let weak = $window.as_weak();
        $window.on_minimize(move || {
            if let Some(w) = weak.upgrade() {
                w.window().set_minimized(true);
            }
        });
    }};
    ($window:expr, resizable) => {{
        wire_frame!($window);
        let weak = $window.as_weak();
        $window.on_toggle_maximize(move || {
            if let Some(w) = weak.upgrade() {
                let maximized = frame::toggle_maximize(w.window());
                w.set_is_maximized(maximized);
            }
        });
        let weak = $window.as_weak();
        $window.on_start_resize(move |edge| {
            if let Some(w) = weak.upgrade() {
                frame::start_resize(w.window(), edge);
            }
        });
    }};
}

impl App {
    pub fn new(store: Store, path: PathBuf) -> anyhow::Result<Rc<App>> {
        let theme = match store.setting("theme")?.as_deref() {
            Some("hot-dog-stand") => ThemeName::HotDogStand,
            _ => ThemeName::Aero,
        };
        let dark = store.setting("dark")?.as_deref() == Some("true");
        let system_frame = store.setting("system_frame")?.as_deref() == Some("true");

        let app = Rc::new(App {
            store: RefCell::new(store),
            path,
            zone: TimeZone::system(),
            main: MainWindow::new()?,
            data: RefCell::default(),
            query: RefCell::default(),
            shown: RefCell::default(),
            assignee_filters: RefCell::default(),
            label_filters: RefCell::default(),
            ticket_windows: RefCell::default(),
            next_new_key: Cell::new(-1),
            cascade: Cell::new(0),
            people_window: RefCell::default(),
            labels_window: RefCell::default(),
            about_window: RefCell::default(),
            theme: Cell::new(theme),
            dark: Cell::new(dark),
            system_frame: Cell::new(system_frame),
        });
        app.wire_main();
        app.reload();
        Ok(app)
    }

    pub fn run(&self, started: std::time::Instant) -> anyhow::Result<()> {
        if timing() {
            let mut first = true;
            self.main.window().set_rendering_notifier(move |state, _| {
                if first && matches!(state, slint::RenderingState::AfterRendering) {
                    first = false;
                    eprintln!("timing: first frame after {:.1?}", started.elapsed());
                }
            })?;
        }
        self.main.show()?;
        slint::run_event_loop()?;
        Ok(())
    }

    /// Shows a message in the yellow bar of the main window.
    fn report(&self, error: impl std::fmt::Display) {
        self.main.set_error(error.to_string().into());
    }

    fn wire_main(self: &Rc<Self>) {
        let main = &self.main;
        style!(self, main);
        main.set_system_frame(self.system_frame.get());
        wire_menu!(self, main);
        main.set_workspace_name(
            self.store
                .borrow()
                .setting("workspace_name")
                .ok()
                .flatten()
                .unwrap_or_default()
                .into(),
        );
        wire_frame!(main, resizable);
        main.on_close_window(|| {
            let _ = slint::quit_event_loop();
        });
        main.window().on_close_requested(|| {
            let _ = slint::quit_event_loop();
            slint::CloseRequestResponse::HideWindow
        });

        let weak = Rc::downgrade(self);
        let call = move |f: fn(&Rc<App>)| {
            let weak = weak.clone();
            move || {
                if let Some(app) = weak.upgrade() {
                    f(&app);
                }
            }
        };
        main.on_filter_changed(call(|app| {
            app.read_filter();
            app.refresh_list();
        }));
        main.on_new_ticket(call(|app| app.open_ticket(None)));
        main.on_export_json(call(|app| app.export_json()));
        main.on_export_csv(call(|app| app.export_csv()));
        main.on_show_people(call(|app| app.show_people()));
        main.on_show_labels(call(|app| app.show_labels()));
        main.on_show_about(call(|app| app.show_about()));

        let weak = Rc::downgrade(self);
        main.on_sort(move |c| {
            let Some(app) = weak.upgrade() else { return };
            let Some(Some(column)) = COLUMNS.get(c as usize) else {
                return;
            };
            let sort = app.query.borrow().sort.clicked(*column);
            app.query.borrow_mut().sort = sort;
            app.refresh_list();
        });
        let weak = Rc::downgrade(self);
        main.on_open_ticket(move |row| {
            let Some(app) = weak.upgrade() else { return };
            let id = {
                let shown = app.shown.borrow();
                let data = app.data.borrow();
                shown.get(row as usize).map(|&i| data.tickets[i].id)
            };
            if let Some(id) = id {
                app.open_ticket(Some(id));
            }
        });
        let weak = Rc::downgrade(self);
        main.on_set_theme(move |theme| {
            if let Some(app) = weak.upgrade() {
                app.set_theme(theme);
            }
        });
        let weak = Rc::downgrade(self);
        main.on_set_dark(move |on| {
            if let Some(app) = weak.upgrade() {
                app.set_dark(on);
            }
        });
        let weak = Rc::downgrade(self);
        main.on_set_system_frame(move |on| {
            if let Some(app) = weak.upgrade() {
                app.set_system_frame(on);
            }
        });
    }

    /// Reads the store again, and shows the result in the main window.
    fn reload(&self) {
        let result = (|| -> hds_store::Result<Data> {
            let store = self.store.borrow();
            Ok(Data {
                tickets: store.tickets()?,
                people: store.people()?,
                labels: store.labels()?,
            })
        })();
        match result {
            Ok(data) => *self.data.borrow_mut() = data,
            Err(error) => self.report(error),
        }
        self.rebuild_filters();
        self.refresh_list();
    }

    /// Makes the lists of the filters from the people and the labels, and
    /// keeps the choice of each filter when it is still there.
    fn rebuild_filters(&self) {
        let data = self.data.borrow();
        let query = self.query.borrow();

        let mut assignees = vec![AssigneeFilter::Anyone, AssigneeFilter::Nobody];
        let mut names: Vec<String> = vec!["Anyone".into(), "Nobody".into()];
        for person in &data.people {
            assignees.push(AssigneeFilter::Person(person.id));
            names.push(if person.is_active {
                person.name.clone()
            } else {
                format!("{} (hidden)", person.name)
            });
        }
        self.main.set_assignee_choices(strings(names));
        self.main
            .set_assignee_filter(index_of(&assignees, &query.filter.assignee));
        *self.assignee_filters.borrow_mut() = assignees;

        let mut labels = vec![None];
        let mut names: Vec<String> = vec!["Any label".into()];
        for label in &data.labels {
            labels.push(Some(label.id));
            names.push(label.name.clone());
        }
        self.main.set_label_choices(strings(names));
        self.main
            .set_label_filter(index_of(&labels, &query.filter.label));
        *self.label_filters.borrow_mut() = labels;
    }

    /// Reads the filter from the main window.
    fn read_filter(&self) {
        let main = &self.main;
        let mut query = self.query.borrow_mut();
        let filter = &mut query.filter;
        filter.status = STATUS_FILTERS
            .get(main.get_status_filter() as usize)
            .copied()
            .unwrap_or_default();
        filter.priority = (main.get_priority_filter() as usize)
            .checked_sub(1)
            .and_then(|i| Priority::ALL.get(i).copied());
        filter.assignee = self
            .assignee_filters
            .borrow()
            .get(main.get_assignee_filter() as usize)
            .copied()
            .unwrap_or_default();
        filter.label = self
            .label_filters
            .borrow()
            .get(main.get_label_filter() as usize)
            .copied()
            .flatten();
        filter.text = main.get_search().to_string();
    }

    /// Shows the tickets that pass the filter, in the order of the sort, and
    /// keeps the selected ticket selected.
    fn refresh_list(&self) {
        let started = std::time::Instant::now();
        let data = self.data.borrow();
        let query = self.query.borrow();
        let selected = {
            let shown = self.shown.borrow();
            usize::try_from(self.main.get_current())
                .ok()
                .and_then(|row| shown.get(row))
                .and_then(|&i| data.tickets.get(i))
                .map(|t| t.id)
        };

        let order = query.run(&data.tickets, &data.people);
        let people = data.person_names();
        let labels: HashMap<i64, &Label> = data.labels.iter().map(|l| (l.id, l)).collect();
        let rows: Vec<TicketRow> = order
            .iter()
            .map(|&i| self.row(&data.tickets[i], &people, &labels))
            .collect();

        let current = selected
            .and_then(|id| order.iter().position(|&i| data.tickets[i].id == id))
            .map_or(-1, |p| p as i32);
        self.main.set_rows(model(rows));
        self.main.set_current(current);
        let column = COLUMNS
            .iter()
            .position(|c| *c == Some(query.sort.column))
            .unwrap_or(0);
        self.main.set_sort_column(column as i32);
        self.main.set_sort_descending(query.sort.descending);
        self.main
            .set_status_text(format::count(order.len(), data.tickets.len()).into());
        *self.shown.borrow_mut() = order;
        if timing() {
            eprintln!(
                "timing: {} of {} tickets shown in {:.1?}",
                self.shown.borrow().len(),
                data.tickets.len(),
                started.elapsed()
            );
        }
    }

    fn row(
        &self,
        ticket: &Ticket,
        people: &HashMap<i64, String>,
        labels: &HashMap<i64, &Label>,
    ) -> TicketRow {
        let assignee = ticket
            .assignee_id
            .and_then(|id| people.get(&id).cloned())
            .unwrap_or_default();
        let chips: Vec<LabelChip> = ticket
            .label_ids
            .iter()
            .filter_map(|id| labels.get(id))
            .map(|l| LabelChip {
                id: l.id as i32,
                name: l.name.as_str().into(),
                color: color(&l.color),
            })
            .collect();
        TicketRow {
            id: ticket.id as i32,
            cells: strings([
                ticket.id.to_string(),
                ticket.title.to_string(),
                ticket.status.label().to_owned(),
                ticket.priority.label().to_owned(),
                assignee,
                String::new(),
                format::time(ticket.updated_at, &self.zone),
            ]),
            priority_level: Priority::ALL
                .iter()
                .position(|p| *p == ticket.priority)
                .unwrap_or(1) as i32,
            labels: model(chips),
            closed: ticket.status == Status::Closed,
        }
    }

    fn set_theme(&self, theme: ThemeName) {
        self.theme.set(theme);
        let value = match theme {
            ThemeName::Aero => "aero",
            ThemeName::HotDogStand => "hot-dog-stand",
        };
        if let Err(error) = self.store.borrow().set_setting("theme", value) {
            self.report(error);
        }
        self.restyle();
    }

    fn set_dark(&self, dark: bool) {
        self.dark.set(dark);
        let value = if dark { "true" } else { "false" };
        if let Err(error) = self.store.borrow().set_setting("dark", value) {
            self.report(error);
        }
        self.restyle();
    }

    /// Gives every open window the current theme.
    fn restyle(&self) {
        style!(self, &self.main);
        for view in self.ticket_windows.borrow().values() {
            style!(self, &view.window);
        }
        if let Some(w) = &*self.people_window.borrow() {
            style!(self, w);
        }
        if let Some(w) = &*self.labels_window.borrow() {
            style!(self, w);
        }
        if let Some(w) = &*self.about_window.borrow() {
            style!(self, w);
        }
    }

    fn set_system_frame(&self, on: bool) {
        self.system_frame.set(on);
        let value = if on { "true" } else { "false" };
        if let Err(error) = self.store.borrow().set_setting("system_frame", value) {
            self.report(error);
        }
        self.main.set_system_frame(on);
        self.restyle();
        for view in self.ticket_windows.borrow().values() {
            view.window.set_system_frame(on);
        }
        if let Some(w) = &*self.people_window.borrow() {
            w.set_system_frame(on);
        }
        if let Some(w) = &*self.labels_window.borrow() {
            w.set_system_frame(on);
        }
        if let Some(w) = &*self.about_window.borrow() {
            w.set_system_frame(on);
        }
    }

    // The ticket windows.

    /// Opens the window of a ticket, or of a new ticket. A ticket that has a
    /// window already comes to the front.
    fn open_ticket(self: &Rc<Self>, id: Option<i64>) {
        if let Some(id) = id
            && let Some(view) = self.ticket_windows.borrow().get(&id)
        {
            frame::raise(view.window.window());
            return;
        }
        let window = match TicketWindow::new() {
            Ok(window) => window,
            Err(error) => return self.report(error),
        };
        let key = id.unwrap_or_else(|| {
            let key = self.next_new_key.get();
            self.next_new_key.set(key - 1);
            key
        });
        style!(self, &window);
        wire_menu!(self, &window);
        window.set_system_frame(self.system_frame.get());
        wire_frame!(window, resizable);
        self.wire_ticket(&window, key);

        self.ticket_windows.borrow_mut().insert(
            key,
            TicketView {
                window: window.clone_strong(),
                id,
                assignees: Vec::new(),
                labels: Vec::new(),
            },
        );
        self.fill_ticket(key);
        self.place(window.window());
        if let Err(error) = window.show() {
            self.report(error);
        }
    }

    fn wire_ticket(self: &Rc<Self>, window: &TicketWindow, key: i64) {
        let weak: Weak<App> = Rc::downgrade(self);
        let call = move |f: fn(&App, i64)| {
            let weak = weak.clone();
            move || {
                if let Some(app) = weak.upgrade() {
                    f(&app, key);
                }
            }
        };
        window.on_cancel(call(App::close_ticket));
        window.on_add_comment(call(App::add_comment));
        window.on_save_comment(call(App::save_comment));
        window.on_cancel_comment(call(App::cancel_comment));
        window
            .global::<Markdown<'_>>()
            .on_render(|text| format::markdown(&text));
        window.global::<Markdown<'_>>().on_open_link(|link| {
            if format::link_is_safe(&link) {
                // A system with no browser leaves the link as it is.
                let _ = open::that_detached(link.as_str());
            }
        });
        let weak = window.as_weak();
        window.on_edit_comment(move |comment_id| {
            let Some(window) = weak.upgrade() else { return };
            let body = window
                .get_history()
                .iter()
                .find(|e| e.is_comment && e.comment_id == comment_id)
                .map(|e| e.text);
            if let Some(body) = body {
                window.set_comment_draft(body);
                window.set_editing_comment(comment_id);
            }
        });
        window.on_close_or_reopen(call(App::close_or_reopen));
        let close = call(App::close_ticket);
        window.window().on_close_requested(move || {
            close();
            slint::CloseRequestResponse::HideWindow
        });
        let weak = Rc::downgrade(self);
        window.on_save(move |then_close| {
            if let Some(app) = weak.upgrade() {
                app.save_ticket(key, then_close);
            }
        });
    }

    /// Puts a new window down and to the right of the main window, one step
    /// further than the window before it. After eight steps it starts again.
    fn place(&self, window: &slint::Window) {
        const STEPS: u8 = 8;
        let step = self.cascade.get();
        self.cascade.set((step + 1) % STEPS);
        let origin = self.main.window().position();
        let scale = self.main.window().scale_factor();
        let offset = (f32::from(step + 1) * 32.0 * scale) as i32;
        window.set_position(slint::PhysicalPosition::new(
            origin.x + offset,
            origin.y + offset,
        ));
    }

    /// Puts the data of the ticket into its window.
    fn fill_ticket(&self, key: i64) {
        let mut windows = self.ticket_windows.borrow_mut();
        let Some(view) = windows.get_mut(&key) else {
            return;
        };
        let data = self.data.borrow();
        let ticket = view
            .id
            .and_then(|id| data.tickets.iter().find(|t| t.id == id));
        let window = &view.window;

        // "Nobody", then each active person, then the person of the ticket
        // when that person is hidden.
        let assignee = ticket.and_then(|t| t.assignee_id);
        let mut ids = vec![None];
        let mut names = vec!["Nobody".to_owned()];
        for person in &data.people {
            if person.is_active || Some(person.id) == assignee {
                ids.push(Some(person.id));
                names.push(person.name.clone());
            }
        }
        window.set_assignee_choices(strings(names));
        window.set_assignee_index(index_of(&ids, &assignee));
        view.assignees = ids;

        let chips: Vec<LabelChip> = data
            .labels
            .iter()
            .map(|l| LabelChip {
                id: l.id as i32,
                name: l.name.as_str().into(),
                color: color(&l.color),
            })
            .collect();
        let checked: Vec<bool> = data
            .labels
            .iter()
            .map(|l| ticket.is_some_and(|t| t.label_ids.contains(&l.id)))
            .collect();
        window.set_all_labels(model(chips));
        window.set_label_checked(model(checked));
        view.labels = data.labels.iter().map(|l| l.id).collect();

        let Some(ticket) = ticket else {
            window.set_ticket_id(0);
            window.set_priority_index(1);
            return;
        };
        window.set_ticket_id(ticket.id as i32);
        window.set_title_text(ticket.title.as_str().into());
        window.set_description(ticket.description.as_str().into());
        window.set_status_index(index_of(&Status::ALL, &ticket.status));
        window.set_priority_index(index_of(&Priority::ALL, &ticket.priority));
        window.set_dates_text(format::dates(ticket, &self.zone).into());
        window.set_history(model(self.history(ticket.id, &data)));
    }

    /// The comments and the events of a ticket, oldest first.
    fn history(&self, ticket_id: i64, data: &Data) -> Vec<HistoryEntry> {
        let store = self.store.borrow();
        let (events, comments) = match (store.events(ticket_id), store.comments(ticket_id)) {
            (Ok(e), Ok(c)) => (e, c),
            (Err(error), _) | (_, Err(error)) => {
                self.report(error);
                return Vec::new();
            }
        };
        let people = data.person_names();
        let labels = data.label_names();
        let names = Names {
            people: &people,
            labels: &labels,
        };
        let mut entries: Vec<(jiff::Timestamp, i64, HistoryEntry)> = events
            .iter()
            .map(|e| {
                let entry = HistoryEntry {
                    is_comment: false,
                    comment_id: 0,
                    when: format::time(e.created_at, &self.zone).into(),
                    text: format::event_text(e, &names).into(),
                    edited: false,
                };
                (e.created_at, 0, entry)
            })
            .collect();
        entries.extend(comments.iter().map(|c| {
            let entry = HistoryEntry {
                is_comment: true,
                comment_id: c.id as i32,
                when: format::time(c.created_at, &self.zone).into(),
                text: c.body.as_str().into(),
                edited: c.edited_at.is_some(),
            };
            (c.created_at, 1, entry)
        }));
        entries.sort_by_key(|(at, kind, _)| (*at, *kind));
        entries.into_iter().map(|(_, _, e)| e).collect()
    }

    /// Writes the fields of a ticket window to the store, one edit and one
    /// event for each field that changed.
    fn save_ticket(&self, key: i64, then_close: bool) {
        let Some((window, id, assignees, label_ids)) =
            self.ticket_windows.borrow().get(&key).map(|v| {
                (
                    v.window.clone_strong(),
                    v.id,
                    v.assignees.clone(),
                    v.labels.clone(),
                )
            })
        else {
            return;
        };

        let result = (|| -> hds_store::Result<i64> {
            let title = Title::new(&window.get_title_text())?;
            let description = window.get_description().to_string();
            let priority = Priority::ALL
                .get(window.get_priority_index() as usize)
                .copied()
                .unwrap_or(Priority::Normal);
            let assignee = assignees
                .get(window.get_assignee_index() as usize)
                .copied()
                .flatten();
            let checked: Vec<bool> = window.get_label_checked().iter().collect();
            let wanted: Vec<i64> = label_ids
                .iter()
                .zip(checked.iter().chain(std::iter::repeat(&false)))
                .filter(|(_, on)| **on)
                .map(|(id, _)| *id)
                .collect();

            let mut store = self.store.borrow_mut();
            let ticket = match id {
                Some(id) => store.ticket(id)?,
                None => {
                    let mut new = NewTicket::new(title.clone());
                    new.description = description.clone();
                    new.priority = priority;
                    new.assignee_id = assignee;
                    store.create_ticket(new)?
                }
            };

            let mut edits = vec![
                Edit::Title(title),
                Edit::Description(description),
                Edit::Priority(priority),
                Edit::Assignee(assignee),
            ];
            if id.is_some()
                && let Some(status) = Status::ALL.get(window.get_status_index() as usize)
            {
                edits.push(Edit::Status(*status));
            }
            for label in &label_ids {
                let has = ticket.label_ids.contains(label);
                let wants = wanted.contains(label);
                if wants && !has {
                    edits.push(Edit::AddLabel(*label));
                } else if has && !wants {
                    edits.push(Edit::RemoveLabel(*label));
                }
            }
            for edit in edits {
                store.edit_ticket(ticket.id, edit)?;
            }
            Ok(ticket.id)
        })();

        match result {
            Ok(saved) => {
                window.set_error(SharedString::new());
                self.reload();
                if then_close {
                    self.close_ticket(key);
                } else {
                    if let Some(view) = self.ticket_windows.borrow_mut().get_mut(&key) {
                        view.id = Some(saved);
                    }
                    self.fill_ticket(key);
                }
            }
            Err(error) => window.set_error(error.to_string().into()),
        }
    }

    fn close_or_reopen(&self, key: i64) {
        let Some(window) = self
            .ticket_windows
            .borrow()
            .get(&key)
            .map(|v| v.window.clone_strong())
        else {
            return;
        };
        let closed = Status::ALL.iter().position(|s| *s == Status::Closed);
        let open = Status::ALL.iter().position(|s| *s == Status::Open);
        let target = if window.get_status_index() as usize == closed.unwrap_or(2) {
            open
        } else {
            closed
        };
        window.set_status_index(target.unwrap_or(0) as i32);
        self.save_ticket(key, false);
    }

    fn add_comment(&self, key: i64) {
        let Some((window, Some(id))) = self
            .ticket_windows
            .borrow()
            .get(&key)
            .map(|v| (v.window.clone_strong(), v.id))
        else {
            return;
        };
        let body = window.get_comment_draft().to_string();
        let result = self.store.borrow_mut().add_comment(id, &body);
        match result {
            Ok(_) => {
                window.set_comment_draft(SharedString::new());
                window.set_error(SharedString::new());
                self.reload();
                self.fill_ticket(key);
            }
            Err(error) => window.set_error(error.to_string().into()),
        }
    }

    /// Saves the comment that the box below the history edits.
    fn save_comment(&self, key: i64) {
        let Some(window) = self
            .ticket_windows
            .borrow()
            .get(&key)
            .map(|v| v.window.clone_strong())
        else {
            return;
        };
        let id = i64::from(window.get_editing_comment());
        let body = window.get_comment_draft().to_string();
        let result = self.store.borrow_mut().edit_comment(id, &body);
        match result {
            Ok(_) => {
                self.cancel_comment(key);
                window.set_error(SharedString::new());
                self.reload();
                self.fill_ticket(key);
            }
            Err(error) => window.set_error(error.to_string().into()),
        }
    }

    /// Empties the comment box and leaves the edit of a comment.
    fn cancel_comment(&self, key: i64) {
        if let Some(view) = self.ticket_windows.borrow().get(&key) {
            view.window.set_comment_draft(SharedString::new());
            view.window.set_editing_comment(0);
        }
    }

    fn close_ticket(&self, key: i64) {
        let view = self.ticket_windows.borrow_mut().remove(&key);
        if let Some(view) = view {
            let _ = view.window.hide();
        }
    }

    // The People window.

    fn show_people(self: &Rc<Self>) {
        if let Some(w) = &*self.people_window.borrow() {
            frame::raise(w.window());
            return;
        }
        let window = match PeopleWindow::new() {
            Ok(window) => window,
            Err(error) => return self.report(error),
        };
        style!(self, &window);
        wire_menu!(self, &window);
        window.set_system_frame(self.system_frame.get());
        wire_frame!(window);

        let weak = Rc::downgrade(self);
        let call = move |f: fn(&Rc<App>)| {
            let weak = weak.clone();
            move || {
                if let Some(app) = weak.upgrade() {
                    f(&app);
                }
            }
        };
        window.on_add(call(|app| {
            if app.change_people(|store, _, name| store.add_person(name).map(|p| p.id))
                && let Some(w) = &*app.people_window.borrow()
            {
                w.set_name(SharedString::new());
            }
        }));
        window.on_rename(call(|app| {
            app.change_people(|store, id, name| match id {
                Some(id) => store.rename_person(id, name).map(|p| p.id),
                None => Ok(0),
            });
        }));
        window.on_toggle_active(call(|app| {
            app.change_people(|store, id, _| match id {
                Some(id) => {
                    let active = store.person(id)?.is_active;
                    store.set_person_active(id, !active).map(|p| p.id)
                }
                None => Ok(0),
            });
        }));
        let weak = Rc::downgrade(self);
        window.on_picked(move |row| {
            let Some(app) = weak.upgrade() else { return };
            if let Some(w) = &*app.people_window.borrow()
                && let Some(person) = app.data.borrow().people.get(row as usize)
            {
                w.set_name(person.name.as_str().into());
            }
        });
        let close = call(|app| {
            if let Some(w) = app.people_window.borrow_mut().take() {
                let _ = w.hide();
            }
        });
        window.on_close_window(close.clone());
        window.window().on_close_requested(move || {
            close();
            slint::CloseRequestResponse::HideWindow
        });

        *self.people_window.borrow_mut() = Some(window.clone_strong());
        self.fill_people(None);
        if let Err(error) = window.show() {
            self.report(error);
        }
    }

    fn fill_people(&self, select: Option<i64>) {
        let Some(window) = self
            .people_window
            .borrow()
            .as_ref()
            .map(|w| w.clone_strong())
        else {
            return;
        };
        let data = self.data.borrow();
        let rows: Vec<PersonRow> = data
            .people
            .iter()
            .map(|p| PersonRow {
                id: p.id as i32,
                name: p.name.as_str().into(),
                active: p.is_active,
            })
            .collect();
        window.set_people(model(rows));
        let current = select
            .and_then(|id| data.people.iter().position(|p| p.id == id))
            .map_or(-1, |p| p as i32);
        window.set_current(current);
    }

    /// Runs one change to the people, then shows the result in every window.
    /// Gives true when the change worked.
    fn change_people(
        &self,
        change: impl FnOnce(&Store, Option<i64>, &str) -> hds_store::Result<i64>,
    ) -> bool {
        let Some(window) = self
            .people_window
            .borrow()
            .as_ref()
            .map(|w| w.clone_strong())
        else {
            return false;
        };
        let selected = usize::try_from(window.get_current())
            .ok()
            .and_then(|row| self.data.borrow().people.get(row).map(|p| p.id));
        let name = window.get_name().to_string();
        let result = change(&self.store.borrow(), selected, &name);
        match result {
            Ok(id) => {
                window.set_error(SharedString::new());
                self.reload();
                self.fill_people(Some(id));
                true
            }
            Err(error) => {
                window.set_error(error.to_string().into());
                false
            }
        }
    }

    // The Labels window.

    fn show_labels(self: &Rc<Self>) {
        if let Some(w) = &*self.labels_window.borrow() {
            frame::raise(w.window());
            return;
        }
        let window = match LabelsWindow::new() {
            Ok(window) => window,
            Err(error) => return self.report(error),
        };
        style!(self, &window);
        wire_menu!(self, &window);
        window.set_system_frame(self.system_frame.get());
        wire_frame!(window);

        let weak = Rc::downgrade(self);
        let call = move |f: fn(&Rc<App>)| {
            let weak = weak.clone();
            move || {
                if let Some(app) = weak.upgrade() {
                    f(&app);
                }
            }
        };
        window.on_add(call(|app| {
            if app.change_labels(|store, _, name, color| store.add_label(name, color).map(|l| l.id))
                && let Some(w) = &*app.labels_window.borrow()
            {
                w.set_name(SharedString::new());
            }
        }));
        window.on_save_changes(call(|app| {
            app.change_labels(|store, id, name, color| match id {
                Some(id) => store.edit_label(id, name, color).map(|l| l.id),
                None => Ok(0),
            });
        }));
        let weak = Rc::downgrade(self);
        window.on_picked(move |row| {
            let Some(app) = weak.upgrade() else { return };
            if let Some(w) = &*app.labels_window.borrow()
                && let Some(label) = app.data.borrow().labels.get(row as usize)
            {
                w.set_name(label.name.as_str().into());
                w.set_color_index(index_of(&LABEL_COLORS, &label.color.as_str()));
            }
        });
        let close = call(|app| {
            if let Some(w) = app.labels_window.borrow_mut().take() {
                let _ = w.hide();
            }
        });
        window.on_close_window(close.clone());
        window.window().on_close_requested(move || {
            close();
            slint::CloseRequestResponse::HideWindow
        });

        *self.labels_window.borrow_mut() = Some(window.clone_strong());
        self.fill_labels(None);
        if let Err(error) = window.show() {
            self.report(error);
        }
    }

    fn fill_labels(&self, select: Option<i64>) {
        let Some(window) = self
            .labels_window
            .borrow()
            .as_ref()
            .map(|w| w.clone_strong())
        else {
            return;
        };
        let data = self.data.borrow();
        let chips: Vec<LabelChip> = data
            .labels
            .iter()
            .map(|l| LabelChip {
                id: l.id as i32,
                name: l.name.as_str().into(),
                color: color(&l.color),
            })
            .collect();
        window.set_labels(model(chips));
        let current = select
            .and_then(|id| data.labels.iter().position(|l| l.id == id))
            .map_or(-1, |p| p as i32);
        window.set_current(current);
    }

    /// Runs one change to the labels, then shows the result in every window.
    /// Gives true when the change worked.
    fn change_labels(
        &self,
        change: impl FnOnce(&Store, Option<i64>, &str, &str) -> hds_store::Result<i64>,
    ) -> bool {
        let Some(window) = self
            .labels_window
            .borrow()
            .as_ref()
            .map(|w| w.clone_strong())
        else {
            return false;
        };
        let selected = usize::try_from(window.get_current())
            .ok()
            .and_then(|row| self.data.borrow().labels.get(row).map(|l| l.id));
        let name = window.get_name().to_string();
        let color = LABEL_COLORS
            .get(window.get_color_index() as usize)
            .copied()
            .unwrap_or(LABEL_COLORS[0]);
        let result = change(&self.store.borrow(), selected, &name, color);
        match result {
            Ok(id) => {
                window.set_error(SharedString::new());
                self.reload();
                self.fill_labels(Some(id));
                true
            }
            Err(error) => {
                window.set_error(error.to_string().into());
                false
            }
        }
    }

    // The About window.

    fn show_about(self: &Rc<Self>) {
        if let Some(w) = &*self.about_window.borrow() {
            frame::raise(w.window());
            return;
        }
        let window = match AboutWindow::new() {
            Ok(window) => window,
            Err(error) => return self.report(error),
        };
        style!(self, &window);
        wire_menu!(self, &window);
        window.set_system_frame(self.system_frame.get());
        window.set_version(env!("CARGO_PKG_VERSION").into());
        window.set_workspace_path(self.path.display().to_string().into());
        let weak = Rc::downgrade(self);
        let close = move || {
            if let Some(app) = weak.upgrade()
                && let Some(w) = app.about_window.borrow_mut().take()
            {
                let _ = w.hide();
            }
        };
        window.on_close_window(close.clone());
        window.window().on_close_requested(move || {
            close();
            slint::CloseRequestResponse::HideWindow
        });
        *self.about_window.borrow_mut() = Some(window.clone_strong());
        if let Err(error) = window.show() {
            self.report(error);
        }
    }

    // Export.

    fn export_json(&self) {
        let name = format!("hotdogstand-{}.json", jiff::Zoned::now().date());
        let Some(path) = save_dialog(&name, "JSON", "json") else {
            return;
        };
        let result = (|| -> anyhow::Result<usize> {
            let snapshot = self.store.borrow().snapshot()?;
            let file = BufWriter::new(File::create(&path)?);
            write_json(&snapshot, file)?;
            Ok(snapshot.tickets.len())
        })();
        self.after_export(result, &path);
    }

    fn export_csv(&self) {
        let name = format!("hotdogstand-{}.csv", jiff::Zoned::now().date());
        let Some(path) = save_dialog(&name, "CSV", "csv") else {
            return;
        };
        let result = (|| -> anyhow::Result<usize> {
            let data = self.data.borrow();
            let shown = self.shown.borrow();
            let file = BufWriter::new(File::create(&path)?);
            write_csv(
                shown.iter().map(|&i| &data.tickets[i]),
                &data.people,
                &data.labels,
                file,
            )?;
            Ok(shown.len())
        })();
        self.after_export(result, &path);
    }

    fn after_export(&self, result: anyhow::Result<usize>, path: &Path) {
        match result {
            Ok(count) => {
                self.main.set_error(SharedString::new());
                let tickets = if count == 1 { "ticket" } else { "tickets" };
                self.main.set_status_text(
                    format!("Exported {count} {tickets} to {}", path.display()).into(),
                );
            }
            Err(error) => self.report(format!("The export failed: {error}")),
        }
    }
}

/// True when `HOTDOGSTAND_TIMING` is set. Then the application writes the
/// time of its first frame and of each refresh of the list to stderr.
fn timing() -> bool {
    std::env::var_os("HOTDOGSTAND_TIMING").is_some()
}

fn save_dialog(name: &str, kind: &str, extension: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Export")
        .set_file_name(name)
        .add_filter(kind, &[extension])
        .save_file()
}
