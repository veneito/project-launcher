#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod appearance;
mod assets;
mod projects;

use gpui::{prelude::*, *};
use gpui_component::{
    Disableable, Root, Selectable, Theme, ThemeMode, TitleBar,
    button::{Button, ButtonVariants},
    input::{Input, InputEvent, InputState},
};
use projects::Project;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Projects,
    Settings,
}

struct Launcher {
    query: Entity<InputState>,
    surface_color: Entity<InputState>,
    accent_color: Entity<InputState>,
    roots: Vec<PathBuf>,
    projects: Vec<Project>,
    page: Page,
    kind: Option<&'static str>,
    message: String,
    appearance: appearance::Appearance,
}

impl Launcher {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let appearance = appearance::Appearance::load();
        appearance.apply(window);
        appearance.apply_theme(cx);
        let surface_color = cx.new(|cx| {
            InputState::new(window, cx).default_value(format!("#{:06X}", appearance.surface))
        });
        let accent_color = cx.new(|cx| {
            InputState::new(window, cx).default_value(format!("#{:06X}", appearance.accent))
        });
        let roots = projects::roots();
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("Search projects"));
        cx.subscribe_in(&query, window, |view, _, event, _, cx| match event {
            InputEvent::PressEnter { .. } => {
                if let Some(&first) = view.visible(cx).first() {
                    view.open(first, cx);
                }
            }
            InputEvent::Change { .. } => cx.notify(),
            _ => {}
        })
        .detach();
        Self {
            query,
            surface_color,
            accent_color,
            projects: projects::scan(&roots),
            roots,
            page: Page::Projects,
            kind: None,
            message: String::new(),
            appearance,
        }
    }

    fn rescan(&mut self) {
        self.projects = projects::scan(&self.roots);
        // A filter for a type that no longer exists would show an empty list.
        if self.kind.is_some_and(|k| !self.projects.iter().any(|p| p.kind == k)) {
            self.kind = None;
        }
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

    fn folders_panel(&self, cx: &mut Context<Self>) -> Div {
        let mut panel = panel("Folders to scan")
            .child("Every folder directly inside these is listed as a project.");
        for (index, root) in self.roots.iter().enumerate() {
            panel = panel.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(div().flex_1().min_w_0().child(root.display().to_string()))
                    .child(
                        Button::new(SharedString::from(format!("remove-{index}")))
                            .label("Remove")
                            .on_click(cx.listener(move |view, _, _, cx| {
                                view.roots.remove(index);
                                view.save_roots(cx);
                            })),
                    ),
            );
        }
        if self.roots.is_empty() {
            panel = panel.child(div().text_color(rgb(0xd7b26d)).child("No folders. Add one to see projects."));
        }
        panel.child(
            Button::new("add-folder")
                .label("Add folders…")
                .on_click(cx.listener(|view, _, _, cx| view.pick_folder(cx))),
        )
    }

    fn appearance_panel(&self, cx: &mut Context<Self>) -> Div {
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
        let mut palettes = div().flex().flex_wrap().gap_2();
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
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.set_palette(surface, accent, window, cx);
                    })),
            );
        }
        panel("Look")
            .child("Glass lets the desktop show through; Solid is opaque and uses the least graphics work.")
            .child(modes)
            .child("Glass tint · lower is more transparent")
            .child(strength)
            .child("Theme colors")
            .child(palettes)
            .child("Background tint · dark hex color")
            .child(Input::new(&self.surface_color))
            .child("Accent · hex color")
            .child(Input::new(&self.accent_color))
            .child(Button::new("apply-colors").label("Apply colors").on_click(cx.listener(
                |view, _, window, cx| {
                    let colors = appearance::parse_color(&view.surface_color.read(cx).value())
                        .and_then(|surface| {
                            if !appearance::dark_surface(surface) {
                                return Err("Choose a darker background tint so text stays readable.".into());
                            }
                            appearance::parse_color(&view.accent_color.read(cx).value())
                                .map(|accent| (surface, accent))
                        });
                    match colors {
                        Ok((surface, accent)) => view.set_palette(surface, accent, window, cx),
                        Err(error) => {
                            view.message = error;
                            cx.notify();
                        }
                    }
                },
            )))
    }

    /// Indexes into `projects` that pass the kind filter and the search box.
    fn visible(&self, cx: &App) -> Vec<usize> {
        let query = self.query.read(cx).value().to_string();
        (0..self.projects.len())
            .filter(|&i| {
                let p = &self.projects[i];
                self.kind.is_none_or(|k| k == p.kind) && projects::fuzzy(&query, &p.name)
            })
            .collect()
    }

    fn open(&mut self, index: usize, cx: &mut Context<Self>) {
        let project = &self.projects[index];
        self.message = match projects::open_in_code(&project.path) {
            Ok(()) => format!("Opened {} in VS Code", project.name),
            Err(error) => error,
        };
        cx.notify();
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> Div {
        let mut kinds: Vec<(&'static str, usize)> = vec![];
        for project in &self.projects {
            match kinds.iter_mut().find(|(k, _)| *k == project.kind) {
                Some((_, count)) => *count += 1,
                None => kinds.push((project.kind, 1)),
            }
        }
        kinds.sort_by_key(|&(_, count)| std::cmp::Reverse(count));
        let mut nav = div().flex().flex_col().gap_1();
        let entries = std::iter::once((None, "All", self.projects.len()))
            .chain(kinds.into_iter().map(|(k, n)| (Some(k), k, n)));
        for (kind, label, count) in entries {
            let selected = self.page == Page::Projects && self.kind == kind;
            nav = nav.child(
                Button::new(label)
                    .ghost()
                    .selected(selected)
                    .w_full()
                    .justify_start()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(if selected {
                        rgba((self.appearance.accent << 8) | 44)
                    } else {
                        rgba(0x00000000)
                    })
                    .text_color(rgb(if selected { 0xf5f3ee } else { 0xa7a7a7 }))
                    .cursor_pointer()
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.kind = kind;
                        view.page = Page::Projects;
                        cx.notify();
                    }))
                    .label(format!("{label}  ·  {count}")),
            );
        }
        div()
            .w(px(188.))
            .h_full()
            .flex_shrink_0()
            .bg(self.appearance.fill(0x081012))
            .p_4()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                div()
                    .pt_3()
                    .px_3()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xl()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Projects"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0xa7a7a7))
                            .child("Open anything in one step"),
                    ),
            )
            .child(nav)
            .child(div().flex_1())
            .child(
                Button::new("rescan")
                    .label("Rescan folders")
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.rescan();
                        view.message = "Rescanned".into();
                        cx.notify();
                    })),
            )
            .child(
                Button::new("settings")
                    .ghost()
                    .selected(self.page == Page::Settings)
                    .w_full()
                    .justify_start()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(if self.page == Page::Settings {
                        rgba((self.appearance.accent << 8) | 44)
                    } else {
                        rgba(0x00000000)
                    })
                    .text_color(rgb(if self.page == Page::Settings { 0xf5f3ee } else { 0xa7a7a7 }))
                    .cursor_pointer()
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.page = Page::Settings;
                        cx.notify();
                    }))
                    .label("Settings"),
            )
    }
}

fn panel(title: &'static str) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_4()
        .max_w(px(620.))
        .p_5()
        .rounded_lg()
        .bg(rgba(0xffffff07))
        .border_1()
        .border_color(rgba(0xffffff12))
        .child(div().text_lg().child(title))
}

impl Render for Launcher {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let visible = self.visible(cx);
        let mut list = div()
            .id("projects")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col();
        for &index in &visible {
            let project = &self.projects[index];
            list = list.child(
                div()
                    .id(SharedString::from(format!("project-{index}")))
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .py_2()
                    .cursor_pointer()
                    .border_b_1()
                    .border_color(rgba(0xffffff0d))
                    .hover(|style| style.bg(rgba(0xffffff0d)))
                    .on_click(cx.listener(move |view, _, _, cx| view.open(index, cx)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .font_weight(FontWeight::MEDIUM)
                            .child(project.name.clone()),
                    )
                    .child(
                        div()
                            .w(px(80.))
                            .text_color(rgb(self.appearance.accent))
                            .child(project.kind),
                    )
                    .child(
                        div()
                            .w(px(120.))
                            .text_color(rgb(0xaab6bc))
                            .child(project.branch.clone().unwrap_or_default()),
                    )
                    .child(
                        div()
                            .w(px(90.))
                            .text_color(rgb(0xaab6bc))
                            .child(projects::ago(project.modified)),
                    ),
            );
        }
        if visible.is_empty() {
            list = list.child(
                div()
                    .p_6()
                    .text_color(rgb(0xaab6bc))
                    .child("No projects match."),
            );
        }
        let settings = div()
            .id("settings-page")
            .size_full()
            .overflow_y_scroll()
            .bg(self.appearance.fill(0x0c1417))
            .p_6()
            .flex()
            .flex_col()
            .gap_4()
            .child(div().text_2xl().font_weight(FontWeight::SEMIBOLD).child("Settings"))
            .child(self.folders_panel(cx))
            .child(self.appearance_panel(cx));
        let projects_page = div()
            .size_full()
            .flex()
            .flex_col()
            .bg(self.appearance.fill(0x0c1417))
            .child(
                div()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(self.kind.unwrap_or("All projects")),
                    )
                    .child(Input::new(&self.query)),
            )
            .child(list);
        let content = if self.page == Page::Settings {
            settings.into_any_element()
        } else {
            projects_page.into_any_element()
        };
        let root_names = self
            .roots
            .iter()
            .map(|root| root.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
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
            .text_color(rgb(0xeeeae2))
            .child(
                TitleBar::new()
                    .bg(self.appearance.fill(0x081012))
                    .child(div().text_sm().child("Project Launcher")),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .p_2()
                    .gap_1()
                    .child(self.sidebar(cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .rounded_lg()
                            .overflow_hidden()
                            .border_1()
                            .border_color(rgba(0xffffff14))
                            .child(content),
                    ),
            )
            .child(
                div()
                    .h(px(44.))
                    .flex_shrink_0()
                    .px_5()
                    .bg(self.appearance.fill(0x101e23))
                    .border_t_1()
                    .border_color(rgba(0xffffff14))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_xs()
                    .text_color(rgb(0xaab6bc))
                    .child(format!("{} of {} projects  ·  {root_names}", visible.len(), self.projects.len()))
                    .child(self.message.clone()),
            )
    }
}

fn main() {
    Application::new()
        .with_assets(assets::Assets)
        .run(|cx: &mut App| {
            gpui_component::init(cx);
            Theme::change(ThemeMode::Dark, None, cx);
            Theme::global_mut(cx).colors.background = rgba(0x00000000).into();
            let bounds = Bounds::centered(None, size(px(980.), px(680.)), cx);
            cx.open_window(
                WindowOptions {
                    window_background: WindowBackgroundAppearance::Transparent,
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(640.), px(420.))),
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
