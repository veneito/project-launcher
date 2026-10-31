use gpui::{AssetSource, SharedString};
use std::borrow::Cow;

pub struct Assets;
impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        let data: &[u8] = match path {
            "icons/window-minimize.svg" => br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path d="M3 8h10" fill="none" stroke="black" stroke-width="1.2"/></svg>"#,
            "icons/window-maximize.svg" => br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><rect x="3.5" y="3.5" width="9" height="9" fill="none" stroke="black" stroke-width="1.2"/></svg>"#,
            "icons/window-restore.svg" => br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path d="M5 3.5h7.5V11M3.5 5.5h7v7h-7z" fill="none" stroke="black" stroke-width="1.2"/></svg>"#,
            "icons/window-close.svg" => br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path d="m4 4 8 8m0-8-8 8" fill="none" stroke="black" stroke-width="1.2"/></svg>"#,
            _ => return Ok(None),
        };
        Ok(Some(Cow::Borrowed(data)))
    }
    fn list(&self, _: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(vec![])
    }
}
