use eframe::egui;
use rfd::FileDialog;
use sqlparser::ast::{Expr, SetExpr, Statement, Value, Values};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
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
                        if let Err(err) = self.parse_sql_to_table(&content) {
                            eprintln!("Failed to parse SQL: {err}");
                        }
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
                    egui::ScrollArea::horizontal().show(ui, |ui| {
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
                });
            }
        });
    }
}

impl MyApp {
    fn parse_sql_to_table(&mut self, sql: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.table_data.clear();
        self.columns.clear();

        let dialect = GenericDialect {};
        let statements = Parser::parse_sql(&dialect, sql)?;

        for stmt in statements {
            if let Statement::Insert { columns, source, .. } = stmt {
                self.columns = columns.iter().map(|c| c.value.clone()).collect();

                if let SetExpr::Values(Values { rows, .. }) = *source.body {
                    for row in rows {
                        let mut cells = Vec::new();
                        for expr in row {
                            match expr {
                                Expr::Value(Value::SingleQuotedString(s)) => cells.push(s),
                                Expr::Value(v) => cells.push(v.to_string()),
                                _ => cells.push(expr.to_string()),
                            }
                        }
                        if !cells.is_empty() {
                            self.table_data.push(cells);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_insert() {
        let sql = "INSERT INTO users (id, name) VALUES (1, 'Alice'), (2, 'Bob');";
        let mut app = MyApp::default();
        app.parse_sql_to_table(sql).unwrap();
        assert_eq!(app.columns, vec!["id".to_string(), "name".to_string()]);
        assert_eq!(app.table_data, vec![
            vec!["1".to_string(), "Alice".to_string()],
            vec!["2".to_string(), "Bob".to_string()],
        ]);
    }
}
