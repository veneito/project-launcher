#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod appearance;
mod assets;
mod projects;

use gpui::{prelude::*, *};
use gpui_component::{
    Root, Selectable, Theme, ThemeMode, TitleBar,
    button::{Button, ButtonVariants},
    input::{Input, InputEvent, InputState},
};
use projects::Project;

struct Launcher {
    query: Entity<InputState>,
    projects: Vec<Project>,
    kind: Option<&'static str>,
    message: String,
    appearance: appearance::Appearance,
}

impl Launcher {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let appearance = appearance::Appearance::load();
        appearance.apply(window);
        appearance.apply_theme(cx);
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
            projects: projects::scan(&projects::roots()),
            kind: None,
            message: String::new(),
            appearance,
        }
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
            let selected = self.kind == kind;
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
                        view.projects = projects::scan(&projects::roots());
                        view.message = "Rescanned".into();
                        cx.notify();
                    })),
            )
    }
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
        let content = div()
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
        let root_names = projects::roots()
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
