//! Commands for a browser tab: its web view over the tab's pane, shown, moved, hidden and closed, and its back, forward, reload and address; the page's loads and title come back as events.

use serde::{Deserialize, Serialize};
use tauri::webview::{NewWindowResponse, PageLoadEvent};
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, Url, WebviewBuilder, WebviewUrl};
use tauri_specta::Event;

/// Where the page shows: CSS pixels in the window.
#[derive(Deserialize, specta::Type, Clone, Copy)]
pub struct BrowserRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BrowserAction {
    Back,
    Forward,
    Reload,
    Open { url: String },
}

#[derive(Serialize, specta::Type, Clone)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BrowserPage {
    Loading { url: String },
    Loaded { url: String },
    Title { title: String },
}

/// A browser tab's page started or finished a load, or changed its title.
#[derive(Serialize, specta::Type, tauri_specta::Event, Clone)]
pub struct BrowserChanged {
    pub tab: String,
    pub page: BrowserPage,
}

fn label(tab: &str) -> String {
    format!("browser-{tab}")
}

/// Only web addresses load: the tab's address comes from agent output and the address field.
fn web_url(url: &str) -> Result<Url, String> {
    let parsed = Url::parse(url).map_err(|_| format!("Cannot open {url}"))?;
    if is_web(&parsed) {
        Ok(parsed)
    } else {
        Err(format!("Cannot open {url}"))
    }
}

fn is_web(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https")
}

fn place(webview: &tauri::Webview, rect: BrowserRect) -> tauri::Result<()> {
    webview.set_position(LogicalPosition::new(rect.x, rect.y))?;
    webview.set_size(LogicalSize::new(rect.width, rect.height))
}

/// Shows the tab's page at `rect`, loading `url` the first time; a tab keeps its page and history while hidden.
#[tauri::command]
#[specta::specta]
pub fn show_browser(
    app: AppHandle,
    tab: String,
    url: String,
    rect: BrowserRect,
) -> Result<(), String> {
    if let Some(webview) = app.get_webview(&label(&tab)) {
        place(&webview, rect).map_err(|error| error.to_string())?;
        return webview.show().map_err(|error| error.to_string());
    }
    let url = web_url(&url)?;
    let window = app.get_window("main").ok_or("The window is gone")?;
    let (loads, titles, popups) = (app.clone(), app.clone(), app.clone());
    let (load_tab, title_tab, popup_tab) = (tab.clone(), tab.clone(), tab.clone());
    let builder = WebviewBuilder::new(label(&tab), WebviewUrl::External(url))
        // An about:blank or other non-web load never reaches the tab, so the saved address stays one that reopens.
        .on_page_load(move |_, payload| {
            if !is_web(payload.url()) {
                return;
            }
            let url = payload.url().to_string();
            let page = match payload.event() {
                PageLoadEvent::Started => BrowserPage::Loading { url },
                PageLoadEvent::Finished => BrowserPage::Loaded { url },
            };
            let _ = BrowserChanged {
                tab: load_tab.clone(),
                page,
            }
            .emit(&loads);
        })
        .on_document_title_changed(move |_, title| {
            let _ = BrowserChanged {
                tab: title_tab.clone(),
                page: BrowserPage::Title { title },
            }
            .emit(&titles);
        })
        // A link that asks for a new window opens in the same tab.
        .on_new_window(move |url, _| {
            if !is_web(&url) {
                return NewWindowResponse::Deny;
            }
            if let Some(webview) = popups.get_webview(&label(&popup_tab)) {
                let _ = webview.navigate(url);
            }
            NewWindowResponse::Deny
        });
    window
        .add_child(
            builder,
            LogicalPosition::new(rect.x, rect.y),
            LogicalSize::new(rect.width, rect.height),
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn move_browser(app: AppHandle, tab: String, rect: BrowserRect) -> Result<(), String> {
    match app.get_webview(&label(&tab)) {
        Some(webview) => place(&webview, rect).map_err(|error| error.to_string()),
        None => Ok(()),
    }
}

#[tauri::command]
#[specta::specta]
pub fn hide_browser(app: AppHandle, tab: String) -> Result<(), String> {
    match app.get_webview(&label(&tab)) {
        Some(webview) => webview.hide().map_err(|error| error.to_string()),
        None => Ok(()),
    }
}

/// The tab went: its page and history go with it.
#[tauri::command]
#[specta::specta]
pub fn close_browser(app: AppHandle, tab: String) -> Result<(), String> {
    match app.get_webview(&label(&tab)) {
        Some(webview) => webview.close().map_err(|error| error.to_string()),
        None => Ok(()),
    }
}

#[tauri::command]
#[specta::specta]
pub fn browse(app: AppHandle, tab: String, action: BrowserAction) -> Result<(), String> {
    let Some(webview) = app.get_webview(&label(&tab)) else {
        return Ok(());
    };
    let done = match action {
        BrowserAction::Back => webview.eval("history.back()"),
        BrowserAction::Forward => webview.eval("history.forward()"),
        BrowserAction::Reload => webview.reload(),
        BrowserAction::Open { url } => webview.navigate(web_url(&url)?),
    };
    done.map_err(|error| error.to_string())
}
