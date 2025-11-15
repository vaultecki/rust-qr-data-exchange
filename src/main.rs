// src/main.rs
use iced::{
    widget::{button, column, container, row, text, text_input, Column},
    Alignment, Element, Length, Task, Theme,
};
use std::path::PathBuf;

mod crypto;
mod qr;

fn main() -> iced::Result {
    tracing_subscriber::fmt::init();
    iced::application("QR Data Exchange", QrApp::update, QrApp::view)
        .theme(QrApp::theme)
        .run_with(QrApp::new)
}

#[derive(Debug, Clone)]
enum Message {
    PasswordChanged(String),
    FilenameChanged(String),
    BrowseFile,
    FileSelected(Option<PathBuf>),
    GenerateQr,
    QrGenerated(Result<QrGenerationResult, String>),
    ReadQrFromFile,
    ReadQrFromString,
    QrReadFromImage(Result<String, String>),
    ShowQrDisplay(QrGenerationResult),
    CloseQrDisplay,
    ShowReadWindow(Option<String>),
    CloseReadWindow,
    DecryptInput(String),
    DecryptAndSave,
    DecryptResult(Result<Vec<u8>, String>),
    SaveDecryptedFile(Vec<u8>),
    FileSaved(Result<(), String>),
}

#[derive(Debug, Clone)]
struct QrGenerationResult {
    qr_text: String,
    qr_image: Vec<u8>, // PNG bytes
}

struct QrApp {
    password: String,
    filename: String,
    qr_display: Option<QrGenerationResult>,
    read_window: Option<ReadWindowState>,
    error_message: Option<String>,
    is_processing: bool,
}

#[derive(Debug, Clone)]
struct ReadWindowState {
    qr_text: String,
    password: String,
}

impl QrApp {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                password: String::new(),
                filename: String::new(),
                qr_display: None,
                read_window: None,
                error_message: None,
                is_processing: false,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PasswordChanged(password) => {
                if password.len() <= 20 {
                    self.password = password;
                }
                Task::none()
            }
            Message::FilenameChanged(filename) => {
                self.filename = filename;
                Task::none()
            }
            Message::BrowseFile => Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .add_filter("All files", &["*"])
                        .add_filter("PNG files", &["png"])
                        .pick_file()
                        .await
                        .map(|f| f.path().to_path_buf())
                },
                Message::FileSelected,
            ),
            Message::FileSelected(Some(path)) => {
                self.filename = path.to_string_lossy().to_string();
                Task::none()
            }
            Message::FileSelected(None) => Task::none(),
            Message::GenerateQr => {
                if self.password.is_empty() {
                    self.error_message = Some("Bitte gib ein Passwort ein.".to_string());
                    return Task::none();
                }
                if self.filename.is_empty() {
                    self.error_message = Some("Bitte wähle eine Datei aus.".to_string());
                    return Task::none();
                }

                let filename = self.filename.clone();
                let password = self.password.clone();
                self.is_processing = true;
                self.error_message = None;

                Task::perform(
                    async move { generate_qr_async(filename, password).await },
                    Message::QrGenerated,
                )
            }
            Message::QrGenerated(Ok(result)) => {
                self.is_processing = false;
                Task::done(Message::ShowQrDisplay(result))
            }
            Message::QrGenerated(Err(e)) => {
                self.is_processing = false;
                self.error_message = Some(e);
                Task::none()
            }
            Message::ShowQrDisplay(result) => {
                self.qr_display = Some(result);
                Task::none()
            }
            Message::CloseQrDisplay => {
                self.qr_display = None;
                Task::none()
            }
            Message::ReadQrFromFile => {
                if self.password.is_empty() {
                    self.error_message = Some("Bitte gib ein Passwort ein.".to_string());
                    return Task::none();
                }
                if self.filename.is_empty() {
                    self.error_message = Some("Bitte wähle eine Datei aus.".to_string());
                    return Task::none();
                }

                let filename = self.filename.clone();
                Task::perform(
                    async move { read_qr_from_image(filename).await },
                    Message::QrReadFromImage,
                )
            }
            Message::QrReadFromImage(Ok(text)) => Task::done(Message::ShowReadWindow(Some(text))),
            Message::QrReadFromImage(Err(e)) => {
                self.error_message = Some(e);
                Task::none()
            }
            Message::ReadQrFromString => {
                if self.password.is_empty() {
                    self.error_message = Some("Bitte gib ein Passwort ein.".to_string());
                    return Task::none();
                }
                Task::done(Message::ShowReadWindow(None))
            }
            Message::ShowReadWindow(qr_text) => {
                self.read_window = Some(ReadWindowState {
                    qr_text: qr_text.unwrap_or_default(),
                    password: self.password.clone(),
                });
                Task::none()
            }
            Message::CloseReadWindow => {
                self.read_window = None;
                Task::none()
            }
            Message::DecryptInput(text) => {
                if let Some(ref mut window) = self.read_window {
                    window.qr_text = text;
                }
                Task::none()
            }
            Message::DecryptAndSave => {
                if let Some(ref window) = self.read_window {
                    let qr_text = window.qr_text.clone();
                    let password = window.password.clone();

                    Task::perform(
                        async move { decrypt_qr_data(qr_text, password).await },
                        Message::DecryptResult,
                    )
                } else {
                    Task::none()
                }
            }
            Message::DecryptResult(Ok(data)) => Task::done(Message::SaveDecryptedFile(data)),
            Message::DecryptResult(Err(e)) => {
                self.error_message = Some(e);
                Task::none()
            }
            Message::SaveDecryptedFile(data) => Task::perform(
                async move {
                    if let Some(file) = rfd::AsyncFileDialog::new().save_file().await {
                        tokio::fs::write(file.path(), data)
                            .await
                            .map_err(|e| e.to_string())
                    } else {
                        Ok(())
                    }
                },
                Message::FileSaved,
            ),
            Message::FileSaved(Ok(())) => {
                self.read_window = None;
                Task::none()
            }
            Message::FileSaved(Err(e)) => {
                self.error_message = Some(e);
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let main_content = column![
            text("PyQrDataExchange").size(24),
            row![
                text("Password [1-20]:").width(Length::Fixed(120.0)),
                text_input("", &self.password)
                    .on_input(Message::PasswordChanged)
                    .secure(true)
                    .width(Length::Fixed(150.0)),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            row![
                text("Filename:").width(Length::Fixed(120.0)),
                text_input("", &self.filename)
                    .on_input(Message::FilenameChanged)
                    .width(Length::Fixed(250.0)),
                button("Browse").on_press(Message::BrowseFile),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            row![
                button("Read QR").on_press(Message::ReadQrFromFile),
                button("Read String").on_press(Message::ReadQrFromString),
                button(if self.is_processing {
                    "Processing..."
                } else {
                    "Generate QR"
                })
                .on_press_maybe(if self.is_processing {
                    None
                } else {
                    Some(Message::GenerateQr)
                }),
            ]
            .spacing(10),
        ]
            .spacing(20)
            .padding(20);

        let mut content = Column::new().push(main_content);

        if let Some(ref error) = self.error_message {
            content = content.push(
                container(text(error).style(|theme: &Theme| text::Style {
                    color: Some(theme.palette().danger),
                }))
                    .padding(10),
            );
        }

        if let Some(ref qr_result) = self.qr_display {
            content = content.push(qr_display_view(qr_result));
        }

        if let Some(ref read_state) = self.read_window {
            content = content.push(read_window_view(read_state));
        }

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::default()
    }
}

fn qr_display_view(result: &QrGenerationResult) -> Element<Message> {
    let qr_image = iced::widget::image::Handle::from_bytes(result.qr_image.clone());

    container(
        column![
            text("Generierter QR-Code").size(20),
            text_input("", &result.qr_text).width(Length::Fixed(400.0)),
            iced::widget::image(qr_image).width(Length::Fixed(400.0)),
            button("Close").on_press(Message::CloseQrDisplay),
        ]
            .spacing(10)
            .padding(20),
    )
        .style(|theme: &Theme| container::Style {
            background: Some(theme.palette().background.into()),
            border: iced::Border {
                color: theme.palette().primary,
                width: 2.0,
                radius: 5.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn read_window_view(state: &ReadWindowState) -> Element<Message> {
    container(
        column![
            text("QR Data Read").size(20),
            text("Text to convert:"),
            text_input("", &state.qr_text)
                .on_input(Message::DecryptInput)
                .width(Length::Fixed(400.0)),
            row![
                button("Decrypt and Save").on_press(Message::DecryptAndSave),
                button("Close").on_press(Message::CloseReadWindow),
            ]
            .spacing(10),
        ]
            .spacing(10)
            .padding(20),
    )
        .style(|theme: &Theme| container::Style {
            background: Some(theme.palette().background.into()),
            border: iced::Border {
                color: theme.palette().primary,
                width: 2.0,
                radius: 5.0.into(),
            },
            ..Default::default()
        })
        .into()
}

// Async functions for business logic
async fn generate_qr_async(filename: String, password: String) -> Result<QrGenerationResult, String> {
    const MAX_QR_BYTES: usize = 2953;

    let raw_data = tokio::fs::read(&filename)
        .await
        .map_err(|e| format!("Fehler beim Lesen der Datei: {}", e))?;

    let qr_text = qr::processor::QrDataProcessor::serialize(&raw_data, &password)
        .map_err(|e| format!("Fehler bei der Verschlüsselung: {}", e))?;

    if qr_text.len() >= MAX_QR_BYTES {
        return Err(format!(
            "Die Datei ist mit {} Bytes zu groß.",
            qr_text.len()
        ));
    }

    let qr_image = qr::service::generate_qr_image(&qr_text)
        .map_err(|e| format!("Fehler bei der QR-Generierung: {}", e))?;

    Ok(QrGenerationResult { qr_text, qr_image })
}

async fn read_qr_from_image(filename: String) -> Result<String, String> {
    qr::service::read_qr_from_image(&filename)
        .map_err(|e| format!("Fehler beim Lesen des QR-Codes: {}", e))
}

async fn decrypt_qr_data(qr_text: String, password: String) -> Result<Vec<u8>, String> {
    qr::processor::QrDataProcessor::deserialize(&qr_text, &password)
        .map_err(|e| format!("Entschlüsselung fehlgeschlagen: {}", e))
}

