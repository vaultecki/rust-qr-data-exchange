// src/main.rs
use iced::widget::{button, column, text, text_input, Image, container};
use iced::{executor, Application, Command, Element, Length, Settings, Theme};
use std::path::PathBuf;
use std::sync::Arc; // Für thread-sicheres Teilen von Fehlern

mod error;
mod models;
mod services;
// Definiere `qr_data` Modul hier, um die Logik aus `services.rs` zu holen
mod qr_data {
    pub use super::services::{process_data_for_qr, generate_qr_image};
}


pub fn main() -> iced::Result {
    QrApp::run(Settings::default())
}

// 1. Der ZUSTAND der Anwendung
struct QrApp {
    password_input: String,
    file_path: Option<PathBuf>,
    qr_image_handle: Option<iced::widget::image::Handle>,
    status_message: String,
    is_loading: bool,
}

// 2. Die NACHRICHTEN, die den Zustand ändern können
#[derive(Debug, Clone)]
enum Message {
    PasswordChanged(String),
    SelectFile,
    FileSelected(Option<PathBuf>),
    GeneratePressed,
    // Nachricht, die zurückkommt, wenn die Generierung fertig ist
    GenerationComplete(Result<(Vec<u8>, String), Arc<AppError>>),
    // ... weitere Nachrichten für das Dekodieren
}

// 3. Die ANWENDUNGSLOGIK
impl Application for QrApp {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Self {
                password_input: String::new(),
                file_path: None,
                qr_image_handle: None,
                status_message: "Bitte Datei auswählen und Passwort eingeben.".to_string(),
                is_loading: false,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("QR Data Exchange - Iced")
    }

    // Hier wird der Zustand basierend auf Nachrichten aktualisiert
    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::PasswordChanged(password) => {
                self.password_input = password;
                Command::none()
            }
            Message::SelectFile => {
                self.is_loading = true;
                self.status_message = "Öffne Dateidialog...".to_string();
                // Starte eine asynchrone Aktion
                Command::perform(services::pick_file(), Message::FileSelected)
            }
            Message::FileSelected(path) => {
                self.is_loading = false;
                if let Some(p) = path {
                    self.status_message = format!("Datei ausgewählt: {}", p.display());
                    self.file_path = Some(p);
                } else {
                    self.status_message = "Keine Datei ausgewählt.".to_string();
                }
                Command::none()
            }
            Message::GeneratePressed => {
                if let (Some(path), false) = (self.file_path.clone(), self.password_input.is_empty()) {
                    self.is_loading = true;
                    self.status_message = "Generiere QR-Code...".to_string();
                    let password = self.password_input.clone();
                    // Starte die rechenintensive Aufgabe im Hintergrund
                    Command::perform(
                        qr_data::generate_qr_from_file(path, password),
                        |res| Message::GenerationComplete(res.map_err(Arc::new)),
                    )
                } else {
                    self.status_message = "Bitte zuerst eine Datei und ein Passwort angeben.".to_string();
                    Command::none()
                }
            }
            Message::GenerationComplete(Ok((image_bytes, _))) => {
                self.is_loading = false;
                self.status_message = "QR-Code erfolgreich generiert!".to_string();
                self.qr_image_handle = Some(iced::widget::image::Handle::from_memory(image_bytes));
                Command::none()
            }
            Message::GenerationComplete(Err(e)) => {
                self.is_loading = false;
                self.status_message = format!("Fehler: {}", e);
                Command::none()
            }
        }
    }

    // Hier wird die UI basierend auf dem aktuellen Zustand gezeichnet
    fn view(&self) -> Element<Message> {
        let mut controls = column![
            text("Passwort:"),
            text_input("Passwort eingeben", &self.password_input)
                .on_input(Message::PasswordChanged)
                .password(),

            button("Datei für QR-Code auswählen").on_press(Message::SelectFile),
            button("Generieren").on_press(Message::GeneratePressed),
            text(&self.status_message),
        ]
            .spacing(10)
            .padding(20);

        if let Some(handle) = self.qr_image_handle.clone() {
            controls = controls.push(Image::new(handle).width(Length::Fixed(400.0)));
        }

        container(controls).center_x().center_y().into()
    }
}
