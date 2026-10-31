#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod appearance;
mod assets;
mod config;
mod projects;

use config::{Config, KINDS, PRESETS};
use gpui::{prelude::*, *};
use gpui_component::{
    Disableable, Root, Selectable, Sizable, Theme, ThemeMode, TitleBar,
    button::{Button, ButtonVariants},
    input::{Input, InputEvent, InputState},
};
use projects::Project;
use std::path::PathBuf;

const MUTED: u32 = 0xa7b4ba;
const TEXT: u32 = 0xeeeae2;

#[derive(Clone, Copy, PartialEq)]
enum View {
    Home,
    Recent,
    Settings,
}

struct Launcher {
    query: Entity<InputState>,
    project_command: Entity<InputState>,
    kind_commands: Vec<(&'static str, Entity<InputState>)>,
    surface_color: Entity<InputState>,
    accent_color: Entity<InputState>,
    roots: Vec<PathBuf>,
    projects: Vec<Project>,
    config: Config,
    view: View,
    kind: Option<String>,
    selected: Option<String>,
    sidebar_open: bool,
    panel_open: bool,
    library_open: bool,
    message: String,
    appearance: appearance::Appearance,
}

fn key(project: &Project) -> String {
    project.path.to_string_lossy().into_owned()
}

fn icon(name: &str, size: f32, color: impl Into<Hsla>) -> Svg {
    svg()
        .path(format!("icons/{name}.svg"))
        .w(px(size))
        .h(px(size))
        .flex_none()
        .text_color(color)
}

/// A rounded square with the project's first letter, colored from its name.
fn tile(name: &str, size: f32) -> Div {
    let hue = (name.bytes().fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32)) % 360) as f32 / 360.;
    div()
        .w(px(size))
        .h(px(size))
        .flex_none()
        .rounded_md()
        .flex()
        .items_center()
        .justify_center()
        .bg(hsla(hue, 0.38, 0.28, 1.))
        .text_color(hsla(hue, 0.7, 0.85, 1.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_size(px(size * 0.42))
        .child(name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default())
}

fn heading(text: &str) -> Div {
    div()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(MUTED))
        .child(text.to_uppercase())
}

fn card() -> Div {
    div()
        .flex()
        .flex_col()
        .flex_none()
        .rounded_lg()
        .overflow_hidden()
        .border_1()
        .border_color(rgba(0xffffff14))
        .bg(rgba(0xffffff06))
}

/// One settings line: title and note on the left, add the control as a child.
fn setting_row(title: &str, note: &str) -> Div {
    let mut text = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_size(px(15.)).child(title.to_owned()));
    if !note.is_empty() {
        text = text.child(div().text_xs().text_color(rgb(MUTED)).child(note.to_owned()));
    }
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_6()
        .px_5()
        .py_3()
        .border_b_1()
        .border_color(rgba(0xffffff0f))
        .child(text)
}

fn icon_button(id: &'static str, name: &str, on: bool) -> Stateful<Div> {
    div()
        .id(id)
        .w(px(32.))
        .h(px(32.))
        .rounded_md()
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .bg(if on { rgba(0xffffff1a) } else { rgba(0x00000000) })
        .hover(|style| style.bg(rgba(0xffffff1a)))
        .child(icon(name, 18., rgb(if on { TEXT } else { MUTED })))
}

impl Launcher {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let appearance = appearance::Appearance::load();
        appearance.apply(window);
        appearance.apply_theme(cx);
        let config = Config::load();
        let roots = projects::roots();
        let surface_color = cx.new(|cx| {
            InputState::new(window, cx).default_value(format!("#{:06X}", appearance.surface))
        });
        let accent_color = cx.new(|cx| {
            InputState::new(window, cx).default_value(format!("#{:06X}", appearance.accent))
        });
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("Search projects"));
        let project_command = cx.new(|cx| InputState::new(window, cx).placeholder("Uses the type's default"));
        let kind_commands: Vec<(&'static str, Entity<InputState>)> = KINDS
            .iter()
            .map(|&kind| {
                let value = config.default_command(kind);
                (kind, cx.new(|cx| InputState::new(window, cx).default_value(value)))
            })
            .collect();

        cx.subscribe_in(&query, window, |view, _, event, window, cx| match event {
            InputEvent::PressEnter { .. } => {
                let visible = view.visible(cx);
                let chosen = view
                    .selected_project()
                    .map(key)
                    .filter(|path| visible.iter().any(|&i| key(&view.projects[i]) == *path))
                    .or_else(|| visible.first().map(|&i| key(&view.projects[i])));
                if let Some(path) = chosen {
                    view.select(path.clone(), window, cx);
                    view.open(&path, cx);
                }
            }
            InputEvent::Change => cx.notify(),
            _ => {}
        })
        .detach();
        cx.subscribe_in(&project_command, window, |view, _, event, _, cx| {
            if matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                view.save_project_command(cx);
            }
        })
        .detach();
        for (kind, input) in &kind_commands {
            let kind = *kind;
            cx.subscribe_in(input, window, move |view, input, event, _, cx| {
                if matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                    let value = input.read(cx).value().to_string();
                    view.config.commands.insert(kind.into(), value);
                    view.persist(cx);
                }
            })
            .detach();
        }

        Self {
            query,
            project_command,
            kind_commands,
            surface_color,
            accent_color,
            projects: projects::scan(&roots),
            roots,
            config,
            view: View::Home,
            kind: None,
            selected: None,
            sidebar_open: true,
            panel_open: true,
            library_open: true,
            message: String::new(),
            appearance,
        }
    }

    fn kind_of<'a>(&'a self, project: &'a Project) -> &'a str {
        self.config
            .projects
            .get(&key(project))
            .and_then(|c| c.kind.as_deref())
            .unwrap_or(project.kind)
    }

    fn selected_project(&self) -> Option<&Project> {
        let path = self.selected.as_ref()?;
        self.projects.iter().find(|p| key(p) == *path)
    }

    /// The project shown in the bottom bar: the selected one, else the last one opened.
    fn focus_project(&self) -> Option<&Project> {
        self.selected_project().or_else(|| {
            let path = self.config.recent.first()?;
            self.projects.iter().find(|p| key(p) == *path)
        })
    }

    /// Indexes into `projects` that pass the current view, type filter and search box.
    fn visible(&self, cx: &App) -> Vec<usize> {
        let query = self.query.read(cx).value().to_string();
        let matches = |i: usize| {
            let p = &self.projects[i];
            projects::fuzzy(&query, &p.name)
        };
        if self.view == View::Recent {
            return self
                .config
                .recent
                .iter()
                .filter_map(|path| self.projects.iter().position(|p| key(p) == *path))
                .filter(|&i| matches(i))
                .collect();
        }
        (0..self.projects.len())
            .filter(|&i| {
                let p = &self.projects[i];
                self.kind.as_deref().is_none_or(|k| k == self.kind_of(p)) && matches(i)
            })
            .collect()
    }

    fn rescan(&mut self) {
        self.projects = projects::scan(&self.roots);
        if self.kind.as_deref().is_some_and(|k| !self.projects.iter().any(|p| self.kind_of(p) == k)) {
            self.kind = None;
        }
    }

    fn persist(&mut self, cx: &mut Context<Self>) {
        if let Err(error) = self.config.save() {
            self.message = error;
        }
        cx.notify();
    }

    fn select(&mut self, path: String, window: &mut Window, cx: &mut Context<Self>) {
        let command = self
            .config
            .projects
            .get(&path)
            .and_then(|c| c.command.clone())
            .unwrap_or_default();
        self.project_command.update(cx, |input, cx| input.set_value(command, window, cx));
        self.selected = Some(path);
        cx.notify();
    }

    fn open(&mut self, path: &str, cx: &mut Context<Self>) {
        let Some(project) = self.projects.iter().find(|p| key(p) == path) else {
            return;
        };
        let name = project.name.clone();
        let template = self.config.command_for(self.kind_of(project), path);
        self.message = match config::launch(&template, path) {
            Ok(()) => {
                self.config.touch_recent(path);
                format!("Opened {name}")
            }
            Err(error) => error,
        };
        self.persist(cx);
    }

    fn toggle_pin(&mut self, path: &str, cx: &mut Context<Self>) {
        let project = self.config.project_mut(path);
        project.pinned = !project.pinned;
        self.persist(cx);
    }

    fn set_kind(&mut self, path: &str, kind: Option<&str>, cx: &mut Context<Self>) {
        self.config.project_mut(path).kind = kind.map(str::to_owned);
        self.persist(cx);
    }

    fn save_project_command(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.selected.clone() else {
            return;
        };
        let value = self.project_command.read(cx).value().trim().to_owned();
        self.config.project_mut(&path).command = (!value.is_empty()).then_some(value);
        self.persist(cx);
    }

    fn set_project_command(&mut self, template: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.project_command
            .update(cx, |input, cx| input.set_value(template.to_owned(), window, cx));
        self.save_project_command(cx);
    }

    fn set_kind_command(&mut self, kind: &'static str, template: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((_, input)) = self.kind_commands.iter().find(|(k, _)| *k == kind) {
            input.update(cx, |input, cx| input.set_value(template.to_owned(), window, cx));
        }
        self.config.commands.insert(kind.into(), template.to_owned());
        self.persist(cx);
    }

    fn save_roots(&mut self, cx: &mut Context<Self>) {
        self.rescan();
        self.message = match projects::save_roots(&self.roots) {
            Ok(()) => format!("Saved. Scanning {} folder(s).", self.roots.len()),
            Err(error) => error,
        };
        cx.notify();
    }

    fn pick_folder(&mut self, cx: &mut Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: true,
            prompt: Some("Choose folders that contain your projects".into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = picked.await {
                let _ = this.update(cx, |view, cx| {
                    for path in paths {
                        if !view.roots.contains(&path) {
                            view.roots.push(path);
                        }
                    }
                    view.save_roots(cx);
                });
            }
        })
        .detach();
    }

    fn save_appearance(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.appearance.apply(window);
        self.appearance.apply_theme(cx);
        self.message = match self.appearance.save() {
            Ok(()) => "Appearance saved.".into(),
            Err(error) => error,
        };
        cx.notify();
    }

    fn set_palette(&mut self, surface: u32, accent: u32, window: &mut Window, cx: &mut Context<Self>) {
        self.appearance.surface = surface;
        self.appearance.accent = accent;
        self.surface_color
            .update(cx, |input, cx| input.set_value(format!("#{surface:06X}"), window, cx));
        self.accent_color
            .update(cx, |input, cx| input.set_value(format!("#{accent:06X}"), window, cx));
        self.save_appearance(window, cx);
    }

    // ---- navigation -------------------------------------------------------

    fn nav_row(&self, id: impl Into<ElementId>, selected: bool) -> Stateful<Div> {
        div()
            .id(id)
            .flex()
            .items_center()
            .gap_3()
            .px_3()
            .h(px(40.))
            .flex_none()
            .rounded_md()
            .cursor_pointer()
            .text_size(px(15.))
            .text_color(rgb(if selected { 0xf5f3ee } else { 0xc3cdd1 }))
            .bg(if selected {
                rgba((self.appearance.accent << 8) | 0x30)
            } else {
                rgba(0x00000000)
            })
            .hover(|style| style.bg(rgba(0xffffff12)))
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> Div {
        let tone = |selected: bool| rgb(if selected { 0xf5f3ee } else { MUTED });
        let home = self.view == View::Home && self.kind.is_none();
        let mut kinds: Vec<(String, usize)> = vec![];
        for project in &self.projects {
            let kind = self.kind_of(project);
            match kinds.iter_mut().find(|(k, _)| k == kind) {
                Some((_, count)) => *count += 1,
                None => kinds.push((kind.to_owned(), 1)),
            }
        }
        kinds.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

        let mut nav = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                self.nav_row("nav-home", home)
                    .child(icon("home", 18., tone(home)))
                    .child("Home")
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.view = View::Home;
                        view.kind = None;
                        cx.notify();
                    })),
            )
            .child(
                self.nav_row("nav-search", false)
                    .child(icon("search", 18., tone(false)))
                    .child("Search")
                    .on_click(cx.listener(|view, _, window, cx| {
                        view.view = View::Home;
                        view.query.update(cx, |input, cx| input.focus(window, cx));
                        cx.notify();
                    })),
            )
            .child(
                self.nav_row("nav-library", false)
                    .child(icon("library", 18., tone(false)))
                    .child(div().flex_1().child("Your Library"))
                    .child(icon(
                        if self.library_open { "chevron-down" } else { "chevron-right" },
                        16.,
                        tone(false),
                    ))
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.library_open = !view.library_open;
                        cx.notify();
                    })),
            );
        if self.library_open {
            for (kind, count) in kinds {
                let selected = self.view == View::Home && self.kind.as_deref() == Some(kind.as_str());
                let pick = kind.clone();
                nav = nav.child(
                    self.nav_row(SharedString::from(format!("kind-{kind}")), selected)
                        .pl(px(46.))
                        .child(div().flex_1().child(kind))
                        .child(div().text_xs().text_color(rgb(MUTED)).child(count.to_string()))
                        .on_click(cx.listener(move |view, _, _, cx| {
                            view.view = View::Home;
                            view.kind = Some(pick.clone());
                            cx.notify();
                        })),
                );
            }
        }
        let recent = self.view == View::Recent;
        let settings = self.view == View::Settings;
        nav = nav
            .child(
                self.nav_row("nav-recent", recent)
                    .child(icon("history", 18., tone(recent)))
                    .child("Recent")
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.view = View::Recent;
                        cx.notify();
                    })),
            )
            .child(
                self.nav_row("nav-settings", settings)
                    .child(icon("settings", 18., tone(settings)))
                    .child("Settings")
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.view = View::Settings;
                        cx.notify();
                    })),
            );

        let mut pinned: Vec<&Project> = self.projects.iter().filter(|p| self.config.is_pinned(&key(p))).collect();
        pinned.sort_by_key(|p| p.name.to_lowercase());
        let mut pins = div()
            .id("pinned")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_1();
        for project in &pinned {
            let path = key(project);
            let selected = self.selected.as_deref() == Some(path.as_str());
            pins = pins.child(
                self.nav_row(SharedString::from(format!("pin-{path}")), selected)
                    .h(px(52.))
                    .child(tile(&project.name, 36.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().truncate().child(project.name.clone()))
                            .child(div().text_xs().text_color(rgb(MUTED)).child(self.kind_of(project).to_owned())),
                    )
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.view = View::Home;
                        view.select(path.clone(), window, cx);
                    })),
            );
        }
        if pinned.is_empty() {
            pins = pins.child(
                div()
                    .px_3()
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .child("Pin a project to keep it here."),
            );
        }
        div()
            .w(px(256.))
            .h_full()
            .flex_shrink_0()
            .bg(self.appearance.fill(0x081012))
            .p_3()
            .flex()
            .flex_col()
            .gap_4()
            .child(nav)
            .child(div().px_3().child(heading("Pinned")))
            .child(pins)
    }

    // ---- pages ------------------------------------------------------------

    fn home_page(&self, visible: &[usize], cx: &mut Context<Self>) -> Stateful<Div> {
        let (label, title) = match (self.view, &self.kind) {
            (View::Recent, _) => ("History".to_owned(), "Recent".to_owned()),
            (_, Some(kind)) => (format!("{kind} projects"), kind.clone()),
            _ => ("Library".to_owned(), "All projects".to_owned()),
        };
        let selected = self.selected_project();
        let selected_path = selected.map(key);
        let pinned = selected_path.as_deref().is_some_and(|p| self.config.is_pinned(p));
        let accent = self.appearance.accent;
        let last_opened = self.config.recent.first().cloned();

        let header = div()
            .flex()
            .items_end()
            .gap_6()
            .p_6()
            .child(tile(&title, 128.).rounded_lg())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .min_w_0()
                    .child(heading(&label))
                    .child(div().text_size(px(34.)).font_weight(FontWeight::SEMIBOLD).child(title))
                    .child(div().text_color(rgb(MUTED)).child(format!(
                        "{} project{}  ·  {} folder{}",
                        visible.len(),
                        if visible.len() == 1 { "" } else { "s" },
                        self.roots.len(),
                        if self.roots.len() == 1 { "" } else { "s" },
                    )))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .pt_2()
                            .child(
                                Button::new("open-selected")
                                    .primary()
                                    .label(match selected {
                                        Some(p) => format!("Open {}", p.name),
                                        None => "Open".into(),
                                    })
                                    .disabled(selected.is_none())
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        if let Some(path) = view.selected.clone() {
                                            view.open(&path, cx);
                                        }
                                    })),
                            )
                            .child(
                                Button::new("pin-selected")
                                    .label(if pinned { "Unpin" } else { "Pin" })
                                    .disabled(selected.is_none())
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        if let Some(path) = view.selected.clone() {
                                            view.toggle_pin(&path, cx);
                                        }
                                    })),
                            )
                            .child(Button::new("rescan").label("Rescan").on_click(cx.listener(
                                |view, _, _, cx| {
                                    view.rescan();
                                    view.message = "Rescanned".into();
                                    cx.notify();
                                },
                            ))),
                    ),
            );

        let table_head = div()
            .flex()
            .items_center()
            .h(px(40.))
            .px_6()
            .gap_3()
            .bg(rgba(0xffffff0a))
            .text_color(rgb(MUTED))
            .child(div().w(px(32.)).flex_none().child("#"))
            .child(div().flex_1().child("Title"))
            .child(div().w(px(90.)).flex_none().child("Type"))
            .child(div().w(px(140.)).flex_none().child("Branch"))
            .child(div().w(px(90.)).flex_none().text_right().child("Modified"));

        let mut rows = div().flex().flex_col();
        for (number, &index) in visible.iter().enumerate() {
            let project = &self.projects[index];
            let path = key(project);
            let is_selected = self.selected.as_deref() == Some(path.as_str());
            let is_last = last_opened.as_deref() == Some(path.as_str());
            let mut name = div().flex().items_center().gap_2().min_w_0().child(
                div().truncate().child(project.name.clone()),
            );
            if self.config.is_pinned(&path) {
                name = name.child(icon("heart-filled", 14., rgb(accent)));
            }
            rows = rows.child(
                div()
                    .id(SharedString::from(format!("project-{index}")))
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_6()
                    .h(px(52.))
                    .cursor_pointer()
                    .border_b_1()
                    .border_color(rgba(0xffffff0a))
                    .text_size(px(15.))
                    .bg(if is_selected {
                        rgba((accent << 8) | 0x30)
                    } else {
                        rgba(0x00000000)
                    })
                    .hover(|style| style.bg(rgba(0xffffff0f)))
                    .on_click(cx.listener(move |view, event: &ClickEvent, window, cx| {
                        view.select(path.clone(), window, cx);
                        if event.click_count() >= 2 {
                            view.open(&path, cx);
                        }
                    }))
                    .child(div().w(px(32.)).flex_none().text_color(rgb(MUTED)).child(if is_last {
                        icon("play", 14., rgb(accent)).into_any_element()
                    } else {
                        (number + 1).to_string().into_any_element()
                    }))
                    .child(div().flex_1().min_w_0().child(name))
                    .child(div().w(px(90.)).flex_none().text_color(rgb(MUTED)).child(self.kind_of(project).to_owned()))
                    .child(
                        div()
                            .w(px(140.))
                            .flex_none()
                            .truncate()
                            .text_color(rgb(MUTED))
                            .child(project.branch.clone().unwrap_or_default()),
                    )
                    .child(
                        div()
                            .w(px(90.))
                            .flex_none()
                            .text_right()
                            .text_color(rgb(MUTED))
                            .child(projects::ago(project.modified)),
                    ),
            );
        }
        if visible.is_empty() {
            rows = rows.child(div().p_6().text_color(rgb(MUTED)).child("No projects here."));
        }
        div()
            .id("home")
            .size_full()
            .overflow_y_scroll()
            .bg(self.appearance.fill(0x0c1417))
            .flex()
            .flex_col()
            .child(header)
            .child(div().px_6().pb_4().child(Input::new(&self.query)))
            .child(table_head)
            .child(rows)
    }

    fn settings_page(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let mut folders = card();
        for (index, root) in self.roots.iter().enumerate() {
            folders = folders.child(setting_row(&root.display().to_string(), "").child(
                Button::new(SharedString::from(format!("remove-{index}")))
                    .label("Remove")
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.roots.remove(index);
                        view.save_roots(cx);
                    })),
            ));
        }
        folders = folders.child(
            setting_row("Add folders", "Every folder directly inside a scanned folder is listed as a project.").child(
                Button::new("add-folder")
                    .label("Add folders…")
                    .on_click(cx.listener(|view, _, _, cx| view.pick_folder(cx))),
            ),
        );

        let mut commands = card();
        for (kind, input) in &self.kind_commands {
            let kind = *kind;
            let mut controls = div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().w(px(300.)).child(Input::new(input)));
            for (name, template) in PRESETS {
                controls = controls.child(
                    Button::new(SharedString::from(format!("kp-{kind}-{name}")))
                        .small()
                        .label(name)
                        .on_click(cx.listener(move |view, _, window, cx| {
                            view.set_kind_command(kind, template, window, cx)
                        })),
                );
            }
            commands = commands.child(setting_row(kind, &format!("Opens {kind} projects with")).child(controls));
        }

        let mut modes = div().flex().gap_2();
        for (label, glass) in [("Solid", false), ("Glass", true)] {
            modes = modes.child(
                Button::new(label)
                    .label(label)
                    .selected(self.appearance.glass == glass)
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.appearance.glass = glass;
                        view.save_appearance(window, cx);
                    })),
            );
        }
        let mut strength = div().flex().gap_2();
        for (label, tint) in [("Crystal", 45), ("Light", 100), ("Balanced", 160), ("Deep", 220)] {
            strength = strength.child(
                Button::new(label)
                    .label(label)
                    .selected(self.appearance.tint == tint)
                    .disabled(!self.appearance.glass)
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.appearance.tint = tint;
                        view.save_appearance(window, cx);
                    })),
            );
        }
        let mut palettes = div().flex().flex_wrap().justify_end().gap_2();
        for (name, surface, accent) in [
            ("Teal", 0x0c1417, 0x92c8e5),
            ("Graphite", 0x151515, 0xc4c4ce),
            ("Blue", 0x101a2d, 0x92b9ff),
            ("Plum", 0x211324, 0xd5a8e3),
            ("Forest", 0x101f19, 0x91cba6),
        ] {
            palettes = palettes.child(
                Button::new(name)
                    .label(name)
                    .selected(self.appearance.surface == surface)
                    .on_click(cx.listener(move |view, _, window, cx| view.set_palette(surface, accent, window, cx))),
            );
        }
        let looks = card()
            .child(setting_row("Window material", "Glass lets the desktop show through. Solid is opaque and lightest.").child(modes))
            .child(setting_row("Glass tint", "Lower is more transparent.").child(strength))
            .child(setting_row("Theme", "Preset colors for the whole window.").child(palettes))
            .child(
                setting_row("Background tint", "A dark hex color, such as #0C1417.")
                    .child(div().w(px(160.)).child(Input::new(&self.surface_color))),
            )
            .child(
                setting_row("Accent", "Any hex color. Used for selection and buttons.")
                    .child(div().w(px(160.)).child(Input::new(&self.accent_color))),
            )
            .child(setting_row("Apply custom colors", "").child(
                Button::new("apply-colors").label("Apply").on_click(cx.listener(|view, _, window, cx| {
                    let colors = appearance::parse_color(&view.surface_color.read(cx).value()).and_then(|surface| {
                        if !appearance::dark_surface(surface) {
                            return Err("Choose a darker background tint so text stays readable.".into());
                        }
                        appearance::parse_color(&view.accent_color.read(cx).value()).map(|accent| (surface, accent))
                    });
                    match colors {
                        Ok((surface, accent)) => view.set_palette(surface, accent, window, cx),
                        Err(error) => {
                            view.message = error;
                            cx.notify();
                        }
                    }
                })),
            ));

        div()
            .id("settings-page")
            .size_full()
            .overflow_y_scroll()
            .bg(self.appearance.fill(0x0c1417))
            .p_8()
            .flex()
            .flex_col()
            .gap_4()
            .child(div().text_size(px(34.)).font_weight(FontWeight::SEMIBOLD).child("Settings"))
            .child(div().pt_4().child(heading("Library")))
            .child(folders)
            .child(div().pt_4().child(heading("Open with")))
            .child(div().text_color(rgb(MUTED)).child(
                "The command run when you open a project. {path} becomes the project folder; wrap program paths with spaces in quotes. Press Enter to save.",
            ))
            .child(commands)
            .child(div().pt_4().child(heading("Appearance")))
            .child(looks)
    }

    // ---- right panel and bottom bar --------------------------------------

    fn details_panel(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let mut panel = div()
            .id("details")
            .w(px(320.))
            .h_full()
            .flex_shrink_0()
            .overflow_y_scroll()
            .bg(self.appearance.fill(0x081012))
            .p_4()
            .flex()
            .flex_col()
            .gap_5()
            .child(heading("Details"));

        if let Some(project) = self.selected_project() {
            let path = key(project);
            let current_kind = self.kind_of(project).to_owned();
            let overridden = self.config.projects.get(&path).is_some_and(|c| c.kind.is_some());
            let mut kinds = div().flex().flex_wrap().gap_1();
            let auto_path = path.clone();
            kinds = kinds.child(
                Button::new("pk-auto")
                    .small()
                    .label("Auto")
                    .selected(!overridden)
                    .on_click(cx.listener(move |view, _, _, cx| view.set_kind(&auto_path, None, cx))),
            );
            for kind in KINDS {
                let kind_path = path.clone();
                kinds = kinds.child(
                    Button::new(SharedString::from(format!("pk-{kind}")))
                        .small()
                        .label(kind)
                        .selected(overridden && current_kind == kind)
                        .on_click(cx.listener(move |view, _, _, cx| view.set_kind(&kind_path, Some(kind), cx))),
                );
            }
            let mut presets = div().flex().gap_1();
            for (name, template) in PRESETS {
                presets = presets.child(
                    Button::new(SharedString::from(format!("pc-{name}")))
                        .small()
                        .label(name)
                        .on_click(cx.listener(move |view, _, window, cx| view.set_project_command(template, window, cx))),
                );
            }
            panel = panel
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(tile(&project.name, 56.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(div().text_size(px(17.)).font_weight(FontWeight::SEMIBOLD).child(project.name.clone()))
                                .child(div().text_xs().text_color(rgb(MUTED)).child(format!(
                                    "{}{}",
                                    current_kind,
                                    project.branch.as_ref().map(|b| format!("  ·  {b}")).unwrap_or_default()
                                ))),
                        ),
                )
                .child(div().text_xs().text_color(rgb(MUTED)).child(path.clone()))
                .child(div().flex().flex_col().gap_2().child(heading("Type")).child(kinds))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(heading("Open with"))
                        .child(Input::new(&self.project_command))
                        .child(presets)
                        .child(div().text_xs().text_color(rgb(MUTED)).child(format!(
                            "Empty uses the {} default: {}",
                            current_kind,
                            self.config.default_command(&current_kind)
                        ))),
                );
        } else {
            panel = panel.child(div().text_color(rgb(MUTED)).child("Select a project to see its details and choose how it opens."));
        }

        let mut recent = div().flex().flex_col().gap_1().child(heading("Recently opened"));
        let mut any = false;
        for path in self.config.recent.iter().take(6) {
            let Some(project) = self.projects.iter().find(|p| key(p) == *path) else {
                continue;
            };
            any = true;
            let path = path.clone();
            recent = recent.child(
                self.nav_row(SharedString::from(format!("recent-{path}")), false)
                    .h(px(48.))
                    .child(tile(&project.name, 32.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().truncate().child(project.name.clone()))
                            .child(div().text_xs().text_color(rgb(MUTED)).child(self.kind_of(project).to_owned())),
                    )
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.view = View::Home;
                        view.select(path.clone(), window, cx);
                    })),
            );
        }
        if !any {
            recent = recent.child(div().text_xs().text_color(rgb(MUTED)).child("Nothing opened yet."));
        }
        panel.child(recent)
    }

    fn bottom_bar(&self, total: usize, cx: &mut Context<Self>) -> Div {
        let focus = self.focus_project();
        let path = focus.map(key);
        let left = match focus {
            Some(project) => div()
                .w(px(300.))
                .flex()
                .items_center()
                .gap_3()
                .child(tile(&project.name, 44.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(div().truncate().text_size(px(15.)).child(project.name.clone()))
                        .child(div().truncate().text_xs().text_color(rgb(MUTED)).child(key(project))),
                ),
            None => div().w(px(300.)).text_color(rgb(MUTED)).child("Nothing selected"),
        };
        let reveal = path.clone();
        let terminal = path.clone();
        let open = path.clone();
        let center = div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .gap_2()
            .child(
                Button::new("bar-folder")
                    .label("Folder")
                    .disabled(path.is_none())
                    .on_click(cx.listener(move |view, _, _, cx| {
                        if let Some(path) = &reveal {
                            view.message = config::launch(config::EXPLORER, path).err().unwrap_or_default();
                            cx.notify();
                        }
                    })),
            )
            .child(
                Button::new("bar-terminal")
                    .label("Terminal")
                    .disabled(path.is_none())
                    .on_click(cx.listener(move |view, _, _, cx| {
                        if let Some(path) = &terminal {
                            view.message = config::launch(config::TERMINAL, path).err().unwrap_or_default();
                            cx.notify();
                        }
                    })),
            )
            .child(
                Button::new("bar-open")
                    .primary()
                    .label("Open")
                    .disabled(path.is_none())
                    .on_click(cx.listener(move |view, _, _, cx| {
                        if let Some(path) = &open {
                            view.open(path, cx);
                        }
                    })),
            );
        div()
            .h(px(76.))
            .flex_shrink_0()
            .px_5()
            .bg(self.appearance.fill(0x101e23))
            .border_t_1()
            .border_color(rgba(0xffffff14))
            .flex()
            .items_center()
            .gap_4()
            .child(left)
            .child(center)
            .child(
                div()
                    .w(px(300.))
                    .text_xs()
                    .text_right()
                    .text_color(rgb(MUTED))
                    .child(if self.message.is_empty() {
                        format!("{total} projects")
                    } else {
                        self.message.clone()
                    }),
            )
    }
}

impl Render for Launcher {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let visible = self.visible(cx);
        let page = if self.view == View::Settings {
            self.settings_page(cx)
        } else {
            self.home_page(&visible, cx)
        };
        let mut body = div().flex_1().min_h_0().flex().p_2().gap_1();
        if self.sidebar_open {
            body = body.child(self.sidebar(cx));
        }
        body = body.child(
            div()
                .flex_1()
                .min_w_0()
                .h_full()
                .rounded_lg()
                .overflow_hidden()
                .border_1()
                .border_color(rgba(0xffffff14))
                .child(page),
        );
        if self.panel_open && self.view != View::Settings {
            body = body.child(self.details_panel(cx));
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(if self.appearance.glass {
                rgba(0x00000000)
            } else {
                self.appearance.fill(0x081012)
            })
            .font_family("Segoe UI")
            .text_sm()
            .text_color(rgb(TEXT))
            .child(
                TitleBar::new().bg(self.appearance.fill(0x081012)).child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div().flex().items_center().gap_2().child(
                                icon_button("toggle-sidebar", "sidebar", self.sidebar_open).on_click(cx.listener(
                                    |view, _, _, cx| {
                                        view.sidebar_open = !view.sidebar_open;
                                        cx.notify();
                                    },
                                )),
                            ),
                        )
                        .child(
                            div().flex().items_center().gap_2().child(
                                icon_button("toggle-panel", "panel-right", self.panel_open).on_click(cx.listener(
                                    |view, _, _, cx| {
                                        view.panel_open = !view.panel_open;
                                        cx.notify();
                                    },
                                )),
                            ),
                        ),
                ),
            )
            .child(body)
            .child(self.bottom_bar(self.projects.len(), cx))
    }
}

fn main() {
    Application::new()
        .with_assets(assets::Assets)
        .run(|cx: &mut App| {
            gpui_component::init(cx);
            Theme::change(ThemeMode::Dark, None, cx);
            Theme::global_mut(cx).colors.background = rgba(0x00000000).into();
            let bounds = Bounds::centered(None, size(px(1240.), px(780.)), cx);
            cx.open_window(
                WindowOptions {
                    window_background: WindowBackgroundAppearance::Transparent,
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(860.), px(520.))),
                    titlebar: Some(TitlebarOptions {
                        title: Some("Project Launcher".into()),
                        appears_transparent: true,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(|cx| Launcher::new(window, cx));
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .expect("Could not open the launcher window");
            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            cx.activate(true);
        });
}
