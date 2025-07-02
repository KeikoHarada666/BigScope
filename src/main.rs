use eframe::{egui, App};
use rfd::FileDialog;
use rusqlite::{Connection, Result};
use std::fs;
use std::io::Write;

pub struct SqlViewerApp {
    sql_file_path: Option<String>,
    sql_content: Option<String>,
    error_message: Option<String>,
    conn: Option<Connection>,
    table_names: Vec<String>,
    selected_table: Option<String>,
    table_data: Vec<Vec<String>>,
    table_headers: Vec<String>,
}

impl Default for SqlViewerApp {
    fn default() -> Self {
        Self {
            sql_file_path: None,
            sql_content: None,
            error_message: None,
            conn: None,
            table_names: Vec::new(),
            selected_table: None,
            table_data: Vec::new(),
            table_headers: Vec::new(),
        }
    }
}

impl App for SqlViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Простой SQL Viewer");

            if ui.button("Открыть SQL-файл").clicked() {
                if let Some(path) = FileDialog::new().add_filter("SQL", &["sql"]).pick_file() {
                    self.sql_file_path = Some(path.display().to_string());
                    match fs::read_to_string(&path) {
                        Ok(content) => {
                            self.sql_content = Some(content.clone());
                            self.error_message = None;
                            self.execute_sql(&content);
                        }
                        Err(err) => {
                            self.error_message = Some(format!("Ошибка чтения файла: {}", err));
                        }
                    }
                }
            }

            if let Some(path) = &self.sql_file_path {
                ui.label(format!("Выбран файл: {}", path));
            }

            if let Some(err) = &self.error_message {
                ui.colored_label(egui::Color32::RED, err);
            }

            if let Some(content) = &self.sql_content {
                ui.separator();
                ui.label("Содержимое файла:");
                egui::ScrollArea::vertical()
                    .id_source("sql_scroll_area")
                    .max_height(200.0)
                    .show(ui, |ui| {
                        ui.code(content);
                    });
            }

            if !self.table_names.is_empty() {
                ui.separator();
                ui.label("Обнаруженные таблицы:");

                egui::ComboBox::from_label("Выберите таблицу")
                    .selected_text(
                        self.selected_table
                            .as_deref()
                            .unwrap_or("-- не выбрано --"),
                    )
                    .show_ui(ui, |combo| {
                        for table in &self.table_names {
                            combo.selectable_value(
                                &mut self.selected_table,
                                Some(table.clone()),
                                table,
                            );
                        }
                    });

                let selected_table = self.selected_table.clone();
                if let Some(table) = selected_table {
                    if ui.button("Загрузить таблицу").clicked() {
                        self.load_table_data(&table);
                    }

                    if ui.button("Сохранить в CSV").clicked() {
                        if let Some(path) = FileDialog::new()
                            .set_file_name(&format!("{}.csv", table))
                            .save_file()
                        {
                            match self.export_table_to_csv(&path.display().to_string()) {
                                Ok(_) => self.error_message = None,
                                Err(e) => {
                                    self.error_message = Some(format!("Ошибка сохранения CSV: {}", e));
                                }
                            }
                        }
                    }
                }
            }

            if !self.table_data.is_empty() {
                ui.separator();
                ui.label("Данные таблицы:");

                let scroll_id = format!("data_scroll_area_{}", self.selected_table.as_deref().unwrap_or("table"));

                egui::ScrollArea::both()
                    .id_source(scroll_id)
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        let grid_id = format!("table_grid_{}", self.selected_table.as_deref().unwrap_or("table"));
                        egui::Grid::new(grid_id).striped(true).show(ui, |ui| {
                            for header in &self.table_headers {
                                ui.label(header);
                            }
                            ui.end_row();

                            for row in &self.table_data {
                                for value in row {
                                    ui.label(value);
                                }
                                ui.end_row();
                            }
                        });
                    });
            }
        });
    }
}

impl SqlViewerApp {
    fn execute_sql(&mut self, sql: &str) {
        match Connection::open_in_memory() {
            Ok(conn) => {
                if let Err(e) = conn.execute_batch(sql) {
                    self.error_message = Some(format!("Ошибка выполнения SQL: {}", e));
                    return;
                }
                self.conn = Some(conn);
                self.load_table_names();
            }
            Err(e) => {
                self.error_message = Some(format!("Ошибка подключения к SQLite: {}", e));
            }
        }
    }

    fn load_table_names(&mut self) {
        if let Some(conn) = &self.conn {
            let mut stmt = match conn.prepare(
                "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%';",
            ) {
                Ok(stmt) => stmt,
                Err(e) => {
                    self.error_message = Some(format!("Ошибка при получении таблиц: {}", e));
                    return;
                }
            };

            let rows = stmt.query_map([], |row| row.get(0));
            match rows {
                Ok(names) => {
                    self.table_names = names.filter_map(Result::ok).collect();
                }
                Err(e) => {
                    self.error_message = Some(format!("Ошибка выборки таблиц: {}", e));
                }
            }
        }
    }

    fn load_table_data(&mut self, table_name: &str) {
        self.table_data.clear();
        self.table_headers.clear();

        if let Some(conn) = &self.conn {
            let query = format!("SELECT * FROM {}", table_name);
            let mut stmt = match conn.prepare(&query) {
                Ok(stmt) => stmt,
                Err(e) => {
                    self.error_message = Some(format!("Ошибка подготовки запроса: {}", e));
                    return;
                }
            };

            self.table_headers = stmt
                .column_names()
                .iter()
                .map(|s| s.to_string())
                .collect();

            let column_count = stmt.column_count();
            let rows = stmt.query_map([], move |row| {
                let mut row_vec = Vec::new();
                for i in 0..column_count {
                    let val: Result<String> = row.get(i);
                    match val {
                        Ok(v) => row_vec.push(v),
                        Err(_) => row_vec.push("NULL".to_string()),
                    }
                }
                Ok(row_vec)
            });

            match rows {
                Ok(mapped) => {
                    self.table_data = mapped.filter_map(Result::ok).collect();
                }
                Err(e) => {
                    self.error_message = Some(format!("Ошибка выборки данных: {}", e));
                }
            }
        }
    }

    fn export_table_to_csv(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = std::fs::File::create(path)?;

        // Заголовки
        writeln!(file, "{}", self.table_headers.join(","))?;

        // Данные
        for row in &self.table_data {
            writeln!(file, "{}", row.join(","))?;
        }

        Ok(())
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "SQL Viewer",
        options,
        Box::new(|_cc| Ok(Box::new(SqlViewerApp::default()))),
    )
}
