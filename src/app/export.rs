//! Export functionality for query results

use crate::app::App;
use anyhow::Result;

impl App {
    /// Export results to CSV file
    pub fn export_results_csv(&mut self) {
        if self.result.rows.is_empty() {
            self.error = Some("Nenhum resultado para exportar".to_string());
            return;
        }

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("export_{}.csv", timestamp);

        match self.export_csv(&filename) {
            Ok(()) => {
                self.message = Some(format!("✓ Exportado {} linhas para {}", self.result.rows.len(), filename));
            }
            Err(e) => {
                self.error = Some(format!("Falha na exportação: {}", e));
            }
        }
    }

    /// Export results to JSON file
    pub fn export_results_json(&mut self) {
        if self.result.rows.is_empty() {
            self.error = Some("Nenhum resultado para exportar".to_string());
            return;
        }

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("export_{}.json", timestamp);

        match self.export_json(&filename) {
            Ok(()) => {
                self.message = Some(format!("✓ Exportado {} linhas para {}", self.result.rows.len(), filename));
            }
            Err(e) => {
                self.error = Some(format!("Falha na exportação: {}", e));
            }
        }
    }

    /// Write results to CSV file
    fn export_csv(&self, filename: &str) -> Result<()> {
        let mut wtr = csv::Writer::from_path(filename)?;
        let headers: Vec<String> = self.result.columns.iter().map(|c| c.name.clone()).collect();
        wtr.write_record(&headers)?;
        for row in &self.result.rows {
            let record: Vec<String> = row.iter().map(|c| c.to_string()).collect();
            wtr.write_record(&record)?;
        }
        wtr.flush()?;
        Ok(())
    }

    /// Write results to JSON file
    fn export_json(&self, filename: &str) -> Result<()> {
        let mut rows: Vec<serde_json::Map<String, serde_json::Value>> = Vec::new();
        for row in &self.result.rows {
            let mut obj = serde_json::Map::new();
            for (i, col) in self.result.columns.iter().enumerate() {
                if let Some(cell) = row.get(i) {
                    obj.insert(col.name.clone(), serde_json::Value::String(cell.to_string()));
                }
            }
            rows.push(obj);
        }
        let json = serde_json::to_string_pretty(&rows)?;
        std::fs::write(filename, json)?;
        Ok(())
    }
}
