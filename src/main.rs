// SPDX-License-Identifier: MIT

mod app;
mod config;
mod i18n;

fn main() -> cosmic::iced::Result {
    // Detect the system's preferred languages for localisation.
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
    i18n::init(&requested_languages);

    // Launch the COSMIC applet runtime.
    cosmic::applet::run::<app::AppModel>(())
}
