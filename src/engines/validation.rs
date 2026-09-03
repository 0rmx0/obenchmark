//! Validation module for benchmark results.
//!
//! This module provides comprehensive validation of benchmark results to ensure
//! statistical reliability, consistency, and data quality for professional-grade
//! benchmarking.

use crate::model::result::{BenchResult, BenchScore, ConsistencyIssue, SampleResult, ValidationReport};

/// Maximum acceptable standard deviation percentage for a benchmark to be considered consistent.
/// Value of 15% provides a good balance between strictness and practicality.
const MAX_STD_DEV_PERCENT: f64 = 15.0;

/// Minimum consistency ratio for a benchmark to be considered reliable.
/// Value of 0.8 (80%) ensures good measurement stability.
const MIN_CONSISTENCY_RATIO: f64 = 0.8;

/// Minimum number of samples required for meaningful statistical analysis.
const MIN_SAMPLE_COUNT: usize = 3;

/// Validates a SampleResult for statistical consistency and quality.
///
/// # Arguments
/// * `sample_result` - The SampleResult to validate
/// * `benchmark_name` - Name of the benchmark for error reporting
///
/// # Returns
/// Vector of consistency issues found, if any.
pub fn validate_sample_result(sample_result: &SampleResult, benchmark_name: &str) -> Vec<ConsistencyIssue> {
    let mut issues = Vec::new();

    // Check sample count
    if sample_result.sample_count < MIN_SAMPLE_COUNT {
        issues.push(ConsistencyIssue {
            benchmark_name: benchmark_name.to_string(),
            std_dev_percent: sample_result.std_dev as f64,
            consistency_ratio: sample_result.consistency_ratio(),
            severity: "high".to_string(),
        });
        return issues;
    }

    // Check if sample is consistent based on standard deviation
    if !sample_result.is_consistent(MAX_STD_DEV_PERCENT) {
        let std_dev_percent = if sample_result.value > 0 {
            (sample_result.std_dev / sample_result.value as f64) * 100.0
        } else {
            0.0
        };

        let severity = if std_dev_percent > MAX_STD_DEV_PERCENT * 2.0 {
            "high"
        } else if std_dev_percent > MAX_STD_DEV_PERCENT * 1.5 {
            "medium"
        } else {
            "low"
        };

        issues.push(ConsistencyIssue {
            benchmark_name: benchmark_name.to_string(),
            std_dev_percent,
            consistency_ratio: sample_result.consistency_ratio(),
            severity: severity.to_string(),
        });
    }

    // Check consistency ratio
    let consistency_ratio = sample_result.consistency_ratio();
    if consistency_ratio < MIN_CONSISTENCY_RATIO && consistency_ratio > 0.0 {
        // Only add if we haven't already added a high-severity issue
        if issues.is_empty() || issues.iter().all(|i| i.severity != "high") {
            let severity = if consistency_ratio < 0.5 {
                "high"
            } else if consistency_ratio < 0.7 {
                "medium"
            } else {
                "low"
            };

            let std_dev_percent = if sample_result.value > 0 {
                (sample_result.std_dev / sample_result.value as f64) * 100.0
            } else {
                0.0
            };

            issues.push(ConsistencyIssue {
                benchmark_name: benchmark_name.to_string(),
                std_dev_percent,
                consistency_ratio,
                severity: severity.to_string(),
            });
        }
    }

    issues
}

/// Validates a BenchScore for data quality and statistical reliability.
///
/// # Arguments
/// * `score` - The BenchScore to validate
///
/// # Returns
/// Vector of warning messages for any issues found.
pub fn validate_bench_score(score: &BenchScore) -> Vec<String> {
    let mut warnings = Vec::new();

    // Check for zero or negative scores
    if score.raw_score == 0 {
        warnings.push(format!(
            "Benchmark '{}' has zero raw score - this may indicate a measurement failure",
            score.name
        ));
    }

    // Check weight is reasonable
    if score.weight == 0 {
        warnings.push(format!(
            "Benchmark '{}' has zero weight - it will not contribute to the final score",
            score.name
        ));
    }

    // Validate sample data if present
    if let Some(ref samples) = score.samples {
        let sample_issues = validate_sample_result(samples, &score.name);
        for issue in sample_issues {
            warnings.push(format!(
                "Benchmark '{}' has consistency issues: std_dev={:.1}%, ratio={:.3} (severity: {})",
                issue.benchmark_name, issue.std_dev_percent, issue.consistency_ratio, issue.severity
            ));
        }
    } else {
        warnings.push(format!(
            "Benchmark '{}' has no sample data - statistical analysis not available",
            score.name
        ));
    }

    warnings
}

/// Validates an entire BenchResult for overall quality and consistency.
///
/// # Arguments
/// * `result` - The BenchResult to validate
///
/// # Returns
/// ValidationReport containing all warnings, errors, and consistency issues.
pub fn validate_result(result: &BenchResult) -> ValidationReport {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut consistency_issues = Vec::new();

    // Check if we have any scores
    if result.scores.is_empty() {
        errors.push("No benchmark scores recorded - validation failed".to_string());
        return ValidationReport {
            warnings,
            errors,
            passed: false,
            consistency_issues,
        };
    }

    // Check system information
    if result.system_info.is_none() {
        warnings.push("No system information recorded - cross-platform comparison may be limited".to_string());
    }

    // Check completion statistics
    if result.completed_benchmarks == 0 {
        errors.push("No benchmarks completed successfully".to_string());
    } else if result.failed_benchmarks > 0 {
        warnings.push(format!(
            "{} benchmarks failed to complete - partial results",
            result.failed_benchmarks
        ));
    }

    if result.skipped_benchmarks > 0 {
        warnings.push(format!(
            "{} benchmarks were skipped - results may not be complete",
            result.skipped_benchmarks
        ));
    }

    // Validate each score
    for score in &result.scores {
        let score_warnings = validate_bench_score(score);
        warnings.extend(score_warnings);

        // Collect consistency issues
        if let Some(ref samples) = score.samples {
            let sample_issues = validate_sample_result(samples, &score.name);
            consistency_issues.extend(sample_issues);
        }
    }

    // Check for reasonable score ranges
    for score in &result.scores {
        if score.raw_score > 0 && score.raw_score < 100 {
            warnings.push(format!(
                "Benchmark '{}' has unusually low score ({}) - may indicate measurement issues",
                score.name, score.raw_score
            ));
        }
    }

    // Check for extreme outliers in normalized scores
    if result.cpu_score > 0 && result.cpu_score > 200_000 {
        warnings.push("CPU score appears extremely high - may indicate baseline mismatch".to_string());
    }

    if result.mem_score > 0 && result.mem_score > 200_000 {
        warnings.push("Memory score appears extremely high - may indicate baseline mismatch".to_string());
    }

    if result.disk_score > 0 && result.disk_score > 200_000 {
        warnings.push("Disk score appears extremely high - may indicate baseline mismatch".to_string());
    }

    if result.gfx_score > 0 && result.gfx_score > 200_000 {
        warnings.push("Graphics score appears extremely high - may indicate baseline mismatch".to_string());
    }

    // Determine overall pass/fail
    let passed = errors.is_empty() && consistency_issues.is_empty();

    ValidationReport {
        warnings,
        errors,
        passed,
        consistency_issues,
    }
}

/// Get the overall quality score for a BenchResult (0.0 to 1.0).
///
/// # Arguments
/// * `result` - The BenchResult to evaluate
///
/// # Returns
/// Quality score where 1.0 is perfect and 0.0 indicates serious issues.
pub fn calculate_quality_score(result: &BenchResult) -> f64 {
    let validation = validate_result(result);
    
    if !validation.passed {
        return 0.0;
    }

    // Start with perfect score
    let mut quality: f64 = 1.0;

    // Penalize for warnings
    if !validation.warnings.is_empty() {
        quality -= 0.1; // 10% penalty for any warnings
    }

    // Penalize for consistency issues
    for issue in &validation.consistency_issues {
        match issue.severity.as_str() {
            "high" => quality -= 0.2,
            "medium" => quality -= 0.1,
            "low" => quality -= 0.05,
            _ => quality -= 0.05,
        }
    }

    // Ensure score is within bounds
    quality.max(0.0).min(1.0)
}

/// Check if a result meets professional benchmarking standards.
///
/// # Arguments
/// * `result` - The BenchResult to check
///
/// # Returns
/// True if the result meets professional standards, false otherwise.
pub fn meets_professional_standards(result: &BenchResult) -> bool {
    let validation = validate_result(result);
    
    // Must have no errors
    if !validation.errors.is_empty() {
        return false;
    }

    // Must have no high-severity consistency issues
    for issue in &validation.consistency_issues {
        if issue.severity == "high" {
            return false;
        }
    }

    // Must have completed at least some benchmarks
    result.completed_benchmarks > 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::result::SampleResult;

    #[test]
    fn test_validate_consistent_sample() {
        // Create a consistent sample with low standard deviation
        let samples = vec![1000, 1005, 995, 1002, 998];
        let sample_result = SampleResult::from_samples(samples);
        
        let issues = validate_sample_result(&sample_result, "Test Benchmark");
        assert!(issues.is_empty(), "Consistent sample should have no issues");
    }

    #[test]
    fn test_validate_inconsistent_sample() {
        // Create an inconsistent sample with high standard deviation
        let samples = vec![100, 500, 150, 400, 200];
        let sample_result = SampleResult::from_samples(samples);
        
        let issues = validate_sample_result(&sample_result, "Test Benchmark");
        assert!(!issues.is_empty(), "Inconsistent sample should have issues");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, "high");
    }

    #[test]
    fn test_validate_insufficient_samples() {
        // Create sample with insufficient count
        let samples = vec![100, 105];
        let sample_result = SampleResult::from_samples(samples);
        
        let issues = validate_sample_result(&sample_result, "Test Benchmark");
        assert!(!issues.is_empty(), "Insufficient samples should have issues");
        assert_eq!(issues[0].severity, "high");
    }

    #[test]
    fn test_validate_zero_score() {
        let score = BenchScore {
            name: "Test Benchmark".to_string(),
            raw_score: 0,
            weight: 1,
            samples: None,
            std_dev_percent: None,
            min: None,
            max: None,
        };
        
        let warnings = validate_bench_score(&score);
        assert!(!warnings.is_empty(), "Zero score should generate warning");
        assert!(warnings[0].contains("zero raw score"));
    }

    #[test]
    fn test_validate_zero_weight() {
        let score = BenchScore {
            name: "Test Benchmark".to_string(),
            raw_score: 1000,
            weight: 0,
            samples: None,
            std_dev_percent: None,
            min: None,
            max: None,
        };
        
        let warnings = validate_bench_score(&score);
        assert!(!warnings.is_empty(), "Zero weight should generate warning");
        assert!(warnings[0].contains("zero weight"));
    }

    #[test]
    fn test_validate_empty_result() {
        let result = BenchResult {
            scores: vec![],
            final_score: 0,
            cpu_score: 0,
            mem_score: 0,
            disk_score: 0,
            gfx_score: 0,
            system_info: None,
            errors: vec![],
            validation: None,
            completed_benchmarks: 0,
            failed_benchmarks: 0,
            skipped_benchmarks: 0,
            start_time: None,
            end_time: None,
            duration_seconds: None,
        };
        
        let validation = validate_result(&result);
        assert!(!validation.passed, "Empty result should fail validation");
        assert!(!validation.errors.is_empty(), "Empty result should have errors");
    }

    #[test]
    fn test_quality_score_perfect() {
        let samples = vec![1000, 1005, 995, 1002, 998];
        let sample_result = SampleResult::from_samples(samples);
        
        let result = BenchResult {
            scores: vec![BenchScore {
                name: "Perfect Benchmark".to_string(),
                raw_score: 1000,
                weight: 1,
                samples: Some(sample_result),
                std_dev_percent: Some(1.5),
                min: Some(995),
                max: Some(1005),
            }],
            final_score: 1000,
            cpu_score: 1000,
            mem_score: 0,
            disk_score: 0,
            gfx_score: 0,
            system_info: None,
            errors: vec![],
            validation: None,
            completed_benchmarks: 1,
            failed_benchmarks: 0,
            skipped_benchmarks: 0,
            start_time: None,
            end_time: None,
            duration_seconds: None,
        };
        
        let quality = calculate_quality_score(&result);
        assert_eq!(quality, 1.0, "Perfect result should have quality score of 1.0");
    }

    #[test]
    fn test_meets_professional_standards() {
        let samples = vec![1000, 1005, 995, 1002, 998];
        let sample_result = SampleResult::from_samples(samples);
        
        let result = BenchResult {
            scores: vec![BenchScore {
                name: "Professional Benchmark".to_string(),
                raw_score: 1000,
                weight: 1,
                samples: Some(sample_result),
                std_dev_percent: Some(1.5),
                min: Some(995),
                max: Some(1005),
            }],
            final_score: 1000,
            cpu_score: 1000,
            mem_score: 0,
            disk_score: 0,
            gfx_score: 0,
            system_info: Some(Default::default()),
            errors: vec![],
            validation: None,
            completed_benchmarks: 1,
            failed_benchmarks: 0,
            skipped_benchmarks: 0,
            start_time: None,
            end_time: None,
            duration_seconds: None,
        };
        
        assert!(meets_professional_standards(&result), "Good result should meet professional standards");
    }

    #[test]
    fn test_meets_professional_standards_with_errors() {
        let result = BenchResult {
            scores: vec![],
            final_score: 0,
            cpu_score: 0,
            mem_score: 0,
            disk_score: 0,
            gfx_score: 0,
            system_info: None,
            errors: vec!["Some error".to_string()],
            validation: None,
            completed_benchmarks: 0,
            failed_benchmarks: 1,
            skipped_benchmarks: 0,
            start_time: None,
            end_time: None,
            duration_seconds: None,
        };
        
        assert!(!meets_professional_standards(&result), "Result with errors should not meet professional standards");
    }
}