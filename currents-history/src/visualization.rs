use chrono::{DateTime, Utc};

/// ASCII chart renderer for weather trends
pub struct AsciiChart {
    width: usize,
    height: usize,
}

impl AsciiChart {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }
    
    /// Render a line chart for weather data
    pub fn render_line_chart(&self, data: &[(DateTime<Utc>, f64)], title: &str) -> String {
        if data.is_empty() {
            return format!("No data available for {}", title);
        }
        
        let values: Vec<f64> = data.iter().map(|(_, value)| *value).collect();
        let min_val = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_val = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max_val - min_val;
        
        if range == 0.0 {
            return format!("{}: Constant value {:.1}", title, min_val);
        }
        
        let mut chart = String::new();
        chart.push_str(&format!("{}\n", title));
        chart.push_str(&format!("Range: {:.1} to {:.1}\n", min_val, max_val));
        chart.push_str(&format!("{}\n", "=".repeat(self.width)));
        
        // Create the chart grid
        let mut grid = vec![vec![' '; self.width]; self.height];
        
        // Plot the data points
        for (i, (_, value)) in data.iter().enumerate() {
            if i >= self.width {
                break;
            }
            
            let normalized = (value - min_val) / range;
            let y = ((1.0 - normalized) * (self.height - 1) as f64) as usize;
            
            if y < self.height {
                grid[y][i] = '*';
            }
        }
        
        // Connect points with lines
        for i in 1..data.len().min(self.width) {
            let prev_y = self.get_y_position(data[i-1].1, min_val, range);
            let curr_y = self.get_y_position(data[i].1, min_val, range);
            
            self.draw_line(&mut grid, i-1, prev_y, i, curr_y);
        }
        
        // Render the grid
        for row in &grid {
            chart.push_str(&format!("|{}\n", row.iter().collect::<String>()));
        }
        
        chart.push_str(&format!("{}\n", "-".repeat(self.width)));
        
        // Add time labels
        self.add_time_labels(&mut chart, data);
        
        chart
    }
    
    /// Render a bar chart for daily data
    pub fn render_bar_chart(&self, data: &[(String, f64)], title: &str) -> String {
        if data.is_empty() {
            return format!("No data available for {}", title);
        }
        
        let values: Vec<f64> = data.iter().map(|(_, value)| *value).collect();
        let max_val = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        
        if max_val <= 0.0 {
            return format!("{}: No positive values", title);
        }
        
        let mut chart = String::new();
        chart.push_str(&format!("{}\n", title));
        chart.push_str(&format!("Max: {:.1}\n", max_val));
        chart.push_str(&format!("{}\n", "=".repeat(self.width)));
        
        for (label, value) in data.iter().take(self.width) {
            let bar_length = ((value / max_val) * (self.width - 10) as f64) as usize;
            let bar = "█".repeat(bar_length);
            chart.push_str(&format!("{:<8} {}\n", label, bar));
        }
        
        chart
    }
    
    /// Get Y position for a value
    fn get_y_position(&self, value: f64, min_val: f64, range: f64) -> usize {
        let normalized = (value - min_val) / range;
        ((1.0 - normalized) * (self.height - 1) as f64) as usize
    }
    
    /// Draw a line between two points
    fn draw_line(&self, grid: &mut Vec<Vec<char>>, x1: usize, y1: usize, x2: usize, y2: usize) {
        let dx = (x2 as isize - x1 as isize).abs();
        let dy = (y2 as isize - y1 as isize).abs();
        let sx = if x1 < x2 { 1 } else { -1 };
        let sy = if y1 < y2 { 1 } else { -1 };
        let mut err = dx - dy;
        
        let mut x = x1 as isize;
        let mut y = y1 as isize;
        
        loop {
            if x >= 0 && x < self.width as isize && y >= 0 && y < self.height as isize {
                grid[y as usize][x as usize] = '*';
            }
            
            if x == x2 as isize && y == y2 as isize {
                break;
            }
            
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }
    
    /// Add time labels to the chart
    fn add_time_labels(&self, chart: &mut String, data: &[(DateTime<Utc>, f64)]) {
        if data.is_empty() {
            return;
        }
        
        let step = (data.len() / 5).max(1);
        for (i, (timestamp, _)) in data.iter().enumerate().step_by(step) {
            if i < self.width {
                chart.push_str(&format!("{:<8}", timestamp.format("%m/%d")));
            }
        }
        chart.push('\n');
    }
}

/// Weather trend visualizer
pub struct WeatherVisualizer {
    chart: AsciiChart,
}

impl WeatherVisualizer {
    pub fn new() -> Self {
        Self {
            chart: AsciiChart::new(80, 20),
        }
    }
    
    /// Visualize temperature trends
    pub fn visualize_temperature_trend(&self, data: &[(DateTime<Utc>, f64)]) -> String {
        self.chart.render_line_chart(data, "Temperature Trend (°C)")
    }
    
    /// Visualize humidity trends
    pub fn visualize_humidity_trend(&self, data: &[(DateTime<Utc>, f64)]) -> String {
        self.chart.render_line_chart(data, "Humidity Trend (%)")
    }
    
    /// Visualize wind speed trends
    pub fn visualize_wind_trend(&self, data: &[(DateTime<Utc>, f64)]) -> String {
        self.chart.render_line_chart(data, "Wind Speed Trend (m/s)")
    }
    
    /// Visualize pressure trends
    pub fn visualize_pressure_trend(&self, data: &[(DateTime<Utc>, f64)]) -> String {
        self.chart.render_line_chart(data, "Pressure Trend (hPa)")
    }
    
    /// Visualize daily temperature ranges
    pub fn visualize_daily_ranges(&self, data: &[(String, f64)]) -> String {
        self.chart.render_bar_chart(data, "Daily Temperature Ranges")
    }
}

impl Default for WeatherVisualizer {
    fn default() -> Self {
        Self::new()
    }
}