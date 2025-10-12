//! Enhanced error handling and recovery mechanisms for the orchestrator
//!
//! This module provides comprehensive error handling, recovery strategies,
//! and resilience patterns for the multi-location weather monitoring system.

use anyhow::Result;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::time::{sleep, interval};
use tracing::warn;
use serde::{Deserialize, Serialize};

/// Error recovery strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    /// Retry with exponential backoff
    ExponentialBackoff {
        initial_delay: Duration,
        max_delay: Duration,
        multiplier: f64,
        max_attempts: u32,
    },
    /// Retry with fixed delay
    FixedDelay {
        delay: Duration,
        max_attempts: u32,
    },
    /// Circuit breaker pattern
    CircuitBreaker {
        failure_threshold: u32,
        timeout: Duration,
        half_open_max_calls: u32,
    },
    /// No retry, fail immediately
    FailFast,
}

impl Default for RecoveryStrategy {
    fn default() -> Self {
        Self::ExponentialBackoff {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            multiplier: 2.0,
            max_attempts: 5,
        }
    }
}

/// Error categories for different types of failures
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// Network/API related errors
    Network,
    /// Database/storage related errors
    Storage,
    /// Configuration related errors
    Configuration,
    /// Process/collector related errors
    Process,
    /// System resource related errors
    Resource,
    /// Unknown/unclassified errors
    Unknown,
}

/// Error context with additional metadata
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub category: ErrorCategory,
    pub location_id: Option<String>,
    pub operation: String,
    pub timestamp: Instant,
    pub retry_count: u32,
    pub last_error: Option<String>,
}

/// Circuit breaker state
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,    // Normal operation
    Open,      // Circuit is open, failing fast
    HalfOpen,  // Testing if service is back
}

/// Circuit breaker implementation
#[derive(Debug)]
pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    last_failure_time: Option<Instant>,
    failure_threshold: u32,
    timeout: Duration,
    half_open_calls: u32,
    half_open_max_calls: u32,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, timeout: Duration, half_open_max_calls: u32) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            last_failure_time: None,
            failure_threshold,
            timeout,
            half_open_calls: 0,
            half_open_max_calls,
        }
    }
    
    pub fn can_execute(&self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last_failure) = self.last_failure_time {
                    Instant::now().duration_since(last_failure) >= self.timeout
                } else {
                    true
                }
            }
            CircuitState::HalfOpen => self.half_open_calls < self.half_open_max_calls,
        }
    }
    
    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.state = CircuitState::Closed;
        self.half_open_calls = 0;
    }
    
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(Instant::now());
        
        if self.failure_count >= self.failure_threshold {
            self.state = CircuitState::Open;
        }
    }
    
    pub fn transition_to_half_open(&mut self) {
        if self.state == CircuitState::Open {
            self.state = CircuitState::HalfOpen;
            self.half_open_calls = 0;
        }
    }
    
    pub fn increment_half_open_calls(&mut self) {
        if self.state == CircuitState::HalfOpen {
            self.half_open_calls += 1;
        }
    }
}

/// Error recovery manager
#[derive(Debug)]
pub struct ErrorRecoveryManager {
    strategies: HashMap<String, RecoveryStrategy>,
    circuit_breakers: HashMap<String, CircuitBreaker>,
    error_contexts: HashMap<String, ErrorContext>,
    retry_counts: HashMap<String, u32>,
}

impl ErrorRecoveryManager {
    pub fn new() -> Self {
        Self {
            strategies: HashMap::new(),
            circuit_breakers: HashMap::new(),
            error_contexts: HashMap::new(),
            retry_counts: HashMap::new(),
        }
    }
    
    /// Add a recovery strategy for a specific operation
    pub fn add_strategy(&mut self, operation: &str, strategy: RecoveryStrategy) {
        self.strategies.insert(operation.to_string(), strategy);
    }
    
    /// Add a circuit breaker for a specific service
    pub fn add_circuit_breaker(&mut self, service: &str, failure_threshold: u32, timeout: Duration) {
        let circuit_breaker = CircuitBreaker::new(failure_threshold, timeout, 3);
        self.circuit_breakers.insert(service.to_string(), circuit_breaker);
    }
    
    /// Execute an operation with error recovery
    pub async fn execute_with_recovery<F, T>(
        &mut self,
        operation: &str,
        service: Option<&str>,
        f: F,
    ) -> Result<T>
    where
        F: Fn() -> Result<T> + Send + Sync,
    {
        let operation_key = operation.to_string();
        let _service_key = service.map(|s| s.to_string());
        
        // Check circuit breaker if service is specified
        if let Some(service) = service {
            if let Some(circuit_breaker) = self.circuit_breakers.get(service) {
                if !circuit_breaker.can_execute() {
                    return Err(anyhow::anyhow!("Circuit breaker is open for service: {}", service));
                }
            }
        }
        
        // Get recovery strategy
        let strategy = self.strategies.get(&operation_key)
            .cloned()
            .unwrap_or_default();
        
        // Execute with retry logic
        let result = self.execute_with_retry(&operation_key, &strategy, f).await;
        
        // Update circuit breaker
        if let Some(service) = service {
            if let Some(circuit_breaker) = self.circuit_breakers.get_mut(service) {
                match &result {
                    Ok(_) => circuit_breaker.record_success(),
                    Err(_) => circuit_breaker.record_failure(),
                }
            }
        }
        
        result
    }
    
    /// Execute operation with retry logic
    async fn execute_with_retry<F, T>(
        &mut self,
        operation: &str,
        strategy: &RecoveryStrategy,
        f: F,
    ) -> Result<T>
    where
        F: Fn() -> Result<T> + Send + Sync,
    {
        let mut attempt = 0;
        
        loop {
            attempt += 1;
            
            // Update retry count
            self.retry_counts.insert(operation.to_string(), attempt);
            
            match f() {
                Ok(result) => {
                    // Success - reset retry count
                    self.retry_counts.remove(operation);
                    return Ok(result);
                }
                Err(e) => {
                    // Check if we should retry
                    if !self.should_retry(operation, attempt, strategy) {
                        return Err(e);
                    }
                    
                    // Calculate delay
                    let delay = self.calculate_delay(strategy, attempt);
                    
                    warn!(
                        "Operation {} failed (attempt {}/{}): {}. Retrying in {:?}",
                        operation,
                        attempt,
                        self.get_max_attempts(strategy),
                        e,
                        delay
                    );
                    
                    // Wait before retry
                    sleep(delay).await;
                }
            }
        }
    }
    
    /// Determine if we should retry based on strategy
    fn should_retry(&self, _operation: &str, attempt: u32, strategy: &RecoveryStrategy) -> bool {
        match strategy {
            RecoveryStrategy::FailFast => false,
            RecoveryStrategy::ExponentialBackoff { max_attempts, .. } => attempt < *max_attempts,
            RecoveryStrategy::FixedDelay { max_attempts, .. } => attempt < *max_attempts,
            RecoveryStrategy::CircuitBreaker { .. } => attempt < 3, // Limited retries for circuit breaker
        }
    }
    
    /// Calculate delay based on strategy
    fn calculate_delay(&self, strategy: &RecoveryStrategy, attempt: u32) -> Duration {
        match strategy {
            RecoveryStrategy::ExponentialBackoff {
                initial_delay,
                max_delay,
                multiplier,
                ..
            } => {
                let delay = initial_delay.as_secs_f64() * multiplier.powi(attempt as i32 - 1);
                let delay_secs = delay.min(max_delay.as_secs_f64());
                Duration::from_secs_f64(delay_secs)
            }
            RecoveryStrategy::FixedDelay { delay, .. } => *delay,
            RecoveryStrategy::CircuitBreaker { .. } => Duration::from_secs(1),
            RecoveryStrategy::FailFast => Duration::from_secs(0),
        }
    }
    
    /// Get maximum attempts for strategy
    fn get_max_attempts(&self, strategy: &RecoveryStrategy) -> u32 {
        match strategy {
            RecoveryStrategy::ExponentialBackoff { max_attempts, .. } => *max_attempts,
            RecoveryStrategy::FixedDelay { max_attempts, .. } => *max_attempts,
            RecoveryStrategy::CircuitBreaker { .. } => 3,
            RecoveryStrategy::FailFast => 1,
        }
    }
    
    /// Record error context
    pub fn record_error(&mut self, context: ErrorContext) {
        let key = format!("{}_{}", 
            context.operation, 
            context.location_id.as_deref().unwrap_or("global")
        );
        self.error_contexts.insert(key, context);
    }
    
    /// Get error statistics
    pub fn get_error_stats(&self) -> ErrorStats {
        let total_errors = self.error_contexts.len();
        let retry_counts: Vec<u32> = self.retry_counts.values().cloned().collect();
        let total_retries: u32 = retry_counts.iter().sum();
        let max_retries = retry_counts.iter().max().copied().unwrap_or(0);
        
        ErrorStats {
            total_errors,
            total_retries,
            max_retries,
            active_retries: self.retry_counts.len(),
        }
    }
    
    /// Clear old error contexts
    pub fn cleanup_old_contexts(&mut self, max_age: Duration) {
        let now = Instant::now();
        self.error_contexts.retain(|_, context| {
            now.duration_since(context.timestamp) < max_age
        });
    }
    
    /// Get retry count for an operation
    pub fn get_retry_count(&self, operation: &str) -> u32 {
        self.retry_counts.get(operation).copied().unwrap_or(0)
    }
}

/// Error statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorStats {
    pub total_errors: usize,
    pub total_retries: u32,
    pub max_retries: u32,
    pub active_retries: usize,
}

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub service: String,
    pub status: HealthStatus,
    pub last_check: Instant,
    pub error_count: u32,
    pub last_error: Option<String>,
}

/// Health status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Health monitor for services
#[derive(Debug)]
pub struct HealthMonitor {
    checks: HashMap<String, HealthCheckResult>,
    check_interval: Duration,
}

impl HealthMonitor {
    pub fn new(check_interval: Duration) -> Self {
        Self {
            checks: HashMap::new(),
            check_interval,
        }
    }
    
    /// Start health monitoring
    pub async fn start_monitoring<F>(&mut self, mut check_fn: F)
    where
        F: FnMut(&str) -> Result<()> + Send + 'static,
    {
        let mut interval = interval(self.check_interval);
        
        loop {
            interval.tick().await;
            
            for (service, result) in &mut self.checks {
                match check_fn(service) {
                    Ok(_) => {
                        result.status = HealthStatus::Healthy;
                        result.error_count = 0;
                        result.last_error = None;
                    }
                    Err(e) => {
                        result.error_count += 1;
                        result.last_error = Some(e.to_string());
                        
                        if result.error_count >= 3 {
                            result.status = HealthStatus::Unhealthy;
                        } else if result.error_count >= 1 {
                            result.status = HealthStatus::Degraded;
                        }
                    }
                }
                result.last_check = Instant::now();
            }
        }
    }
    
    /// Add a service to monitor
    pub fn add_service(&mut self, service: String) {
        let service_clone = service.clone();
        self.checks.insert(service, HealthCheckResult {
            service: service_clone,
            status: HealthStatus::Unknown,
            last_check: Instant::now(),
            error_count: 0,
            last_error: None,
        });
    }
    
    /// Get health status for a service
    pub fn get_health(&self, service: &str) -> Option<&HealthCheckResult> {
        self.checks.get(service)
    }
    
    /// Get all health statuses
    pub fn get_all_health(&self) -> &HashMap<String, HealthCheckResult> {
        &self.checks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_circuit_breaker() {
        let mut cb = CircuitBreaker::new(3, Duration::from_secs(10), 2);
        
        // Initially closed
        assert!(cb.can_execute());
        
        // Record failures
        cb.record_failure();
        cb.record_failure();
        assert!(cb.can_execute()); // Still closed
        
        cb.record_failure();
        assert!(!cb.can_execute()); // Now open
        
        // Record success should reset
        cb.record_success();
        assert!(cb.can_execute());
    }
    
    #[test]
    fn test_error_recovery_manager() {
        let mut manager = ErrorRecoveryManager::new();
        
        // Add strategy
        manager.add_strategy("test_op", RecoveryStrategy::FixedDelay {
            delay: Duration::from_secs(1),
            max_attempts: 3,
        });
        
        // Add circuit breaker
        manager.add_circuit_breaker("test_service", 2, Duration::from_secs(5));
        
        // Test error context
        let context = ErrorContext {
            category: ErrorCategory::Network,
            location_id: Some("test".to_string()),
            operation: "test_op".to_string(),
            timestamp: Instant::now(),
            retry_count: 0,
            last_error: None,
        };
        
        manager.record_error(context);
        let stats = manager.get_error_stats();
        assert_eq!(stats.total_errors, 1);
    }
}
