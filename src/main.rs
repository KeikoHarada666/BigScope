use eframe::egui;
use rfd::FileDialog;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "BigScope",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    )
}

#[derive(Default)]
struct MyApp {
    selected_file: Option<PathBuf>,
    table_data: Vec<Vec<String>>, // 2D-таблица
    columns: Vec<String>,          // заголовки
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("BigScope — SQL → Таблица");
            ui.separator();

            if ui.button("Открыть SQL-файл").clicked() {
                if let Some(path) = FileDialog::new().add_filter("SQL", &["sql"]).pick_file() {
                    self.selected_file = Some(path.clone());
                    if let Ok(content) = fs::read_to_string(&path) {
                        self.parse_sql_to_table(&content);
                    }
                }
            }

            if let Some(path) = &self.selected_file {
                ui.label(format!("Выбран файл: {}", path.display()));
            }

            if !self.table_data.is_empty() {
                ui.separator();
                ui.label("Представление как таблицы:");

                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("table_grid")
                        .striped(true)
                        .show(ui, |ui| {
                            // Заголовки
                            for col in &self.columns {
                                ui.label(egui::RichText::new(col).strong());
                            }
                            ui.end_row();

                            // Данные
                            for row in &self.table_data {
                                for cell in row {
                                    ui.label(cell);
                                }
                                ui.end_row();
                            }
                        });
                });
            }
        });
    }
}

impl MyApp {
    fn parse_sql_to_table(&mut self, sql: &str) {
        self.table_data.clear();
        self.columns.clear();

        for line in sql.lines() {
            if line.trim_start().starts_with("INSERT INTO") {
                // Пример: INSERT INTO users (id, name, age) VALUES (1, 'Alice', 30), ...
                let cols_start = line.find('(').unwrap_or(0);
                let cols_end = line.find(')').unwrap_or(cols_start);
                let cols = &line[cols_start + 1..cols_end];
                self.columns = cols.split(',').map(|s| s.trim().to_string()).collect();

                if let Some(values_start) = line.find("VALUES") {
                    let values_str = &line[values_start + 6..].replace("),", ")|"); // Разделяем записи
                    let rows: Vec<&str> = values_str.split('|').collect();

                    for row in rows {
                        let row = row.trim().trim_matches(';').trim_matches('(').trim_matches(')');
                        let cells = row
                            .split(',')
                            .map(|s| s.trim().trim_matches('\'').to_string())
                            .collect::<Vec<String>>();
                        if !cells.is_empty() {
                            self.table_data.push(cells);
                        }
                    }
                }
            }
        }
    }
}