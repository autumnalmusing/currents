use anyhow::{Result, Context};
use currents_core::types::WeatherData;
use rusqlite::{Connection, params};
use std::path::Path;
use chrono::{DateTime, Utc};
use crate::migration::MigrationManager;
use crate::types::{StorageConfig, DatabaseStats, DailyWeather};

/// Weather data storage manager
pub struct WeatherStorage {
    conn: Connection,
    config: StorageConfig,
}

impl WeatherStorage {
    /// Create a new weather storage database
    pub fn new<P: AsRef<Path>>(db_path: P, config: StorageConfig) -> Result<Self> {
        let conn = Connection::open(db_path)
            .context("Failed to open weather storage database")?;
        
        let storage = Self { conn, config };
        storage.init_database()?;
        Ok(storage)
    }
    
    /// Initialize database with migrations
    fn init_database(&self) -> Result<()> {
        let migration_manager = MigrationManager::new(&self.conn);
        migration_manager.migrate()?;
        Ok(())
    }
    
    /// Store weather data in the database
    pub async fn store_weather_data(&self, location_id: &str, data: &WeatherData) -> Result<()> {
        let timestamp = data.timestamp.timestamp();
        
        self.conn.execute(
            "INSERT INTO weather_history (
                location_id, timestamp, temperature, humidity, wind_speed, description,
                precipitation_intensity, precipitation_probability
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                location_id,
                timestamp,
                data.temperature,
                data.humidity,
                data.wind_speed,
                data.description,
                data.precipitation.as_ref().map(|p| &p.intensity),
                data.precipitation.as_ref().map(|p| p.probability)
            ],
        ).context("Failed to insert weather data")?;
        
        // Clean up old data if max_history_days is set
        if self.config.max_history_days > 0 {
            self.cleanup_old_data().await?;
        }
        
        Ok(())
    }
    
    /// Get weather history for the last N days
    pub async fn get_weather_history(&self, days: u32) -> Result<Vec<WeatherData>> {
        let cutoff_time = (Utc::now() - chrono::Duration::days(days as i64)).timestamp();
        
        let mut stmt = self.conn.prepare(
            "SELECT timestamp, temperature, humidity, wind_speed, description,
                    precipitation_intensity, precipitation_probability
             FROM weather_history 
             WHERE timestamp >= ?1 
             ORDER BY timestamp ASC"
        ).context("Failed to prepare weather history query")?;
        
        let rows = stmt.query_map(params![cutoff_time], |row| {
            Ok(WeatherData {
                timestamp: DateTime::from_timestamp(row.get::<_, i64>(0)?, 0)
                    .unwrap_or_else(|| Utc::now()),
                temperature: row.get(1)?,
                humidity: row.get(2)?,
                wind_speed: row.get(3)?,
                description: row.get(4)?,
                precipitation: match (row.get::<_, Option<String>>(5)?, row.get::<_, Option<f64>>(6)?) {
                    (Some(intensity), Some(probability)) => Some(currents_core::types::PrecipitationData {
                        intensity,
                        probability,
                    }),
                    _ => None,
                },
            })
        }).context("Failed to execute weather history query")?;
        
        let mut results = Vec::new();
        for row in rows {
            results.push(row.context("Failed to parse weather data row")?);
        }
        
        Ok(results)
    }
    
    /// Get weather data for a specific metric over time
    pub async fn get_metric_history(&self, metric: &str, days: u32) -> Result<Vec<(DateTime<Utc>, f64)>> {
        let cutoff_time = (Utc::now() - chrono::Duration::days(days as i64)).timestamp();
        
        let column = match metric {
            "temperature" => "temperature",
            "humidity" => "humidity", 
            "wind_speed" => "wind_speed",
            "pressure" => "pressure",
            "visibility" => "visibility",
            "uv_index" => "uv_index",
            "cloud_cover" => "cloud_cover",
            "aqi" => "aqi",
            _ => return Err(anyhow::anyhow!("Unknown metric: {}", metric)),
        };
        
        let mut stmt = self.conn.prepare(&format!(
            "SELECT timestamp, {} FROM weather_history 
             WHERE timestamp >= ?1 AND {} IS NOT NULL
             ORDER BY timestamp ASC",
            column, column
        )).context("Failed to prepare metric history query")?;
        
        let rows = stmt.query_map(params![cutoff_time], |row| {
            Ok((
                DateTime::from_timestamp(row.get::<_, i64>(0)?, 0)
                    .unwrap_or_else(|| Utc::now()),
                row.get(1)?,
            ))
        }).context("Failed to execute metric history query")?;
        
        let mut results = Vec::new();
        for row in rows {
            results.push(row.context("Failed to parse metric data row")?);
        }
        
        Ok(results)
    }
    
    /// Get daily aggregated weather data
    pub async fn get_daily_aggregates(&self, days: u32) -> Result<Vec<DailyWeather>> {
        let cutoff_time = (Utc::now() - chrono::Duration::days(days as i64)).timestamp();
        
        let mut stmt = self.conn.prepare(
            "SELECT 
                DATE(datetime(timestamp, 'unixepoch')) as date,
                MIN(temperature) as min_temp,
                MAX(temperature) as max_temp,
                AVG(temperature) as avg_temp,
                AVG(humidity) as avg_humidity,
                AVG(wind_speed) as avg_wind_speed,
                AVG(pressure) as avg_pressure,
                COUNT(*) as record_count
             FROM weather_history 
             WHERE timestamp >= ?1 
             GROUP BY DATE(datetime(timestamp, 'unixepoch'))
             ORDER BY date ASC"
        ).context("Failed to prepare daily aggregates query")?;
        
        let rows = stmt.query_map(params![cutoff_time], |row| {
            Ok(DailyWeather {
                date: row.get::<_, String>(0)?,
                min_temp: row.get(1)?,
                max_temp: row.get(2)?,
                avg_temp: row.get(3)?,
                avg_humidity: row.get(4)?,
                avg_wind_speed: row.get(5)?,
                avg_pressure: row.get(6)?,
                record_count: row.get(7)?,
            })
        }).context("Failed to execute daily aggregates query")?;
        
        let mut results = Vec::new();
        for row in rows {
            results.push(row.context("Failed to parse daily weather row")?);
        }
        
        Ok(results)
    }
    
    /// Get the most recent weather data
    pub async fn get_latest_weather(&self) -> Result<Option<WeatherData>> {
        let mut stmt = self.conn.prepare(
            "SELECT timestamp, temperature, humidity, wind_speed, description,
                    precipitation_intensity, precipitation_probability
             FROM weather_history 
             ORDER BY timestamp DESC 
             LIMIT 1"
        ).context("Failed to prepare latest weather query")?;
        
        let mut rows = stmt.query_map([], |row| {
            Ok(WeatherData {
                timestamp: DateTime::from_timestamp(row.get::<_, i64>(0)?, 0)
                    .unwrap_or_else(|| Utc::now()),
                temperature: row.get(1)?,
                humidity: row.get(2)?,
                wind_speed: row.get(3)?,
                description: row.get(4)?,
                precipitation: match (row.get::<_, Option<String>>(5)?, row.get::<_, Option<f64>>(6)?) {
                    (Some(intensity), Some(probability)) => Some(currents_core::types::PrecipitationData {
                        intensity,
                        probability,
                    }),
                    _ => None,
                },
            })
        }).context("Failed to execute latest weather query")?;
        
        if let Some(row) = rows.next() {
            Ok(Some(row.context("Failed to parse latest weather row")?))
        } else {
            Ok(None)
        }
    }
    
    /// Get database statistics
    pub async fn get_stats(&self) -> Result<DatabaseStats> {
        let mut stmt = self.conn.prepare(
            "SELECT 
                COUNT(*) as total_records,
                MIN(timestamp) as earliest_timestamp,
                MAX(timestamp) as latest_timestamp,
                AVG(temperature) as avg_temperature,
                MIN(temperature) as min_temperature,
                MAX(temperature) as max_temperature
             FROM weather_history"
        ).context("Failed to prepare stats query")?;
        
        let row = stmt.query_row([], |row| {
            Ok(DatabaseStats {
                total_records: row.get(0)?,
                earliest_timestamp: row.get::<_, Option<i64>>(1)?.map(|ts| 
                    DateTime::from_timestamp(ts, 0).unwrap_or_else(|| Utc::now())
                ),
                latest_timestamp: row.get::<_, Option<i64>>(2)?.map(|ts| 
                    DateTime::from_timestamp(ts, 0).unwrap_or_else(|| Utc::now())
                ),
                avg_temperature: row.get(3)?,
                min_temperature: row.get(4)?,
                max_temperature: row.get(5)?,
            })
        }).context("Failed to execute stats query")?;
        
        Ok(row)
    }
    
    /// Clean up old data based on max_history_days
    async fn cleanup_old_data(&self) -> Result<()> {
        if self.config.max_history_days == 0 {
            return Ok(()); // No cleanup needed
        }
        
        let cutoff_time = (Utc::now() - chrono::Duration::days(self.config.max_history_days as i64)).timestamp();
        
        self.conn.execute(
            "DELETE FROM weather_history WHERE timestamp < ?1",
            params![cutoff_time],
        ).context("Failed to cleanup old data")?;
        
        Ok(())
    }
    
    /// Clean up old data manually (for external use)
    pub async fn cleanup_old_data_manual(&self, keep_days: u32) -> Result<u64> {
        let cutoff_time = (Utc::now() - chrono::Duration::days(keep_days as i64)).timestamp();
        
        let deleted = self.conn.execute(
            "DELETE FROM weather_history WHERE timestamp < ?1",
            params![cutoff_time],
        ).context("Failed to cleanup old data")?;
        
        Ok(deleted as u64)
    }

    /// Get weather data for a specific location and time window
    pub async fn get_weather_data(
        &self, 
        location_id: &str, 
        time_window: Option<chrono::Duration>
    ) -> Result<Vec<WeatherData>> {
        let mut results = Vec::new();
        
        if let Some(duration) = time_window {
            let cutoff_time = (Utc::now() - duration).timestamp();
            let mut stmt = self.conn.prepare(
                "SELECT timestamp, temperature, humidity, wind_speed, description,
                        precipitation_intensity, precipitation_probability
                 FROM weather_history 
                 WHERE location_id = ?1 AND timestamp >= ?2
                 ORDER BY timestamp DESC"
            ).context("Failed to prepare weather data query")?;
            
            let rows = stmt.query_map(params![location_id, cutoff_time], |row| {
                Ok(WeatherData {
                    timestamp: DateTime::from_timestamp(row.get::<_, i64>(0)?, 0)
                        .unwrap_or_else(|| Utc::now()),
                    temperature: row.get(1)?,
                    humidity: row.get(2)?,
                    wind_speed: row.get(3)?,
                    description: row.get(4)?,
                    precipitation: match (row.get::<_, Option<String>>(5)?, row.get::<_, Option<f64>>(6)?) {
                        (Some(intensity), Some(probability)) => Some(currents_core::types::PrecipitationData {
                            intensity,
                            probability,
                        }),
                        _ => None,
                    },
                })
            }).context("Failed to execute weather data query")?;
            
            for row in rows {
                results.push(row.context("Failed to parse weather data row")?);
            }
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT timestamp, temperature, humidity, wind_speed, description,
                        precipitation_intensity, precipitation_probability
                 FROM weather_history 
                 WHERE location_id = ?1
                 ORDER BY timestamp DESC"
            ).context("Failed to prepare weather data query")?;
            
            let rows = stmt.query_map([location_id], |row| {
                Ok(WeatherData {
                    timestamp: DateTime::from_timestamp(row.get::<_, i64>(0)?, 0)
                        .unwrap_or_else(|| Utc::now()),
                    temperature: row.get(1)?,
                    humidity: row.get(2)?,
                    wind_speed: row.get(3)?,
                    description: row.get(4)?,
                    precipitation: match (row.get::<_, Option<String>>(5)?, row.get::<_, Option<f64>>(6)?) {
                        (Some(intensity), Some(probability)) => Some(currents_core::types::PrecipitationData {
                            intensity,
                            probability,
                        }),
                        _ => None,
                    },
                })
            }).context("Failed to execute weather data query")?;
            
            for row in rows {
                results.push(row.context("Failed to parse weather data row")?);
            }
        }
        
        Ok(results)
    }
}