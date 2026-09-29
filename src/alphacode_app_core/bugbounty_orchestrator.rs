//! Bug Bounty Orchestrator — the "beast mode" intelligence layer.
//!
//! This module orchestrates the complete bug bounty workflow from target
//! understanding to vulnerability reporting. It chains tools intelligently,
//! never misses any task, and provides comprehensive attack surface mapping.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// The current phase of the bug bounty workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BountyPhase {
    Reconnaissance,
    AttackSurfaceMapping,
    VulnerabilityDiscovery,
    Exploitation,
    Reporting,
    Complete,
}

impl BountyPhase {
    /// Get the next phase in the workflow.
    pub fn next(&self) -> Option<Self> {
        match self {
            Self::Reconnaissance => Some(Self::AttackSurfaceMapping),
            Self::AttackSurfaceMapping => Some(Self::VulnerabilityDiscovery),
            Self::VulnerabilityDiscovery => Some(Self::Exploitation),
            Self::Exploitation => Some(Self::Reporting),
            Self::Reporting => Some(Self::Complete),
            Self::Complete => None,
        }
    }

    /// Get a human-readable description of the phase.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Reconnaissance => {
                "Reconnaissance: subdomain enumeration, port scanning, service discovery"
            }
            Self::AttackSurfaceMapping => {
                "Attack Surface Mapping: URL collection, content discovery, parameter extraction"
            }
            Self::VulnerabilityDiscovery => {
                "Vulnerability Discovery: template scanning, fuzzing, manual testing"
            }
            Self::Exploitation => {
                "Exploitation: targeted attacks based on discovered vulnerabilities"
            }
            Self::Reporting => "Reporting: comprehensive vulnerability documentation",
            Self::Complete => "Complete: all phases finished",
        }
    }
}

/// A single task in the bug bounty workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BountyTask {
    /// Unique task identifier
    pub id: String,
    /// Human-readable task name
    pub name: String,
    /// Task description
    pub description: String,
    /// The tool to execute for this task
    pub tool: String,
    /// Tool parameters
    pub params: serde_json::Value,
    /// The phase this task belongs to
    pub phase: BountyPhase,
    /// Whether this task has been completed
    pub completed: bool,
    /// Task result (populated after execution)
    pub result: Option<String>,
    /// Dependencies — tasks that must complete before this one
    pub dependencies: Vec<String>,
    /// Priority (lower = higher priority)
    pub priority: u32,
    /// Estimated duration in seconds
    pub estimated_duration: u64,
    /// Retry count
    pub retry_count: u32,
    /// Maximum retries
    pub max_retries: u32,
}

/// The bug bounty orchestrator — manages the complete workflow.
#[derive(Debug, Clone)]
pub struct BugBountyOrchestrator {
    /// The target domain or URL
    pub target: String,
    /// Current phase
    pub current_phase: Arc<RwLock<BountyPhase>>,
    /// All tasks in the workflow
    pub tasks: Arc<RwLock<HashMap<String, BountyTask>>>,
    /// Discovered subdomains
    pub subdomains: Arc<RwLock<HashSet<String>>>,
    /// Discovered URLs
    pub urls: Arc<RwLock<HashSet<String>>>,
    /// Discovered vulnerabilities
    pub vulnerabilities: Arc<RwLock<HashMap<String, String>>>,
    /// Discovered ports
    pub ports: Arc<RwLock<HashSet<u16>>>,
    /// Discovered services
    pub services: Arc<RwLock<HashMap<String, String>>>,
}

impl BugBountyOrchestrator {
    /// Create a new orchestrator for the given target.
    pub fn new(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            current_phase: Arc::new(RwLock::new(BountyPhase::Reconnaissance)),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            subdomains: Arc::new(RwLock::new(HashSet::new())),
            urls: Arc::new(RwLock::new(HashSet::new())),
            vulnerabilities: Arc::new(RwLock::new(HashMap::new())),
            ports: Arc::new(RwLock::new(HashSet::new())),
            services: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if a tool is available and provide alternatives if not.
    ///
    /// Returns the tool to use (either the original or an alternative).
    pub async fn resolve_tool(&self, tool: &str) -> String {
        use crate::alphacode_app_core::bugbounty_doctor::ToolStatus;
        use crate::alphacode_app_core::bugbounty_doctor::which;

        if let ToolStatus::Present { .. } = which(tool) {
            return tool.to_string();
        }

        // Tool not found, try alternatives
        let alternative = match tool {
            "gau" => "waybackurls",
            "waybackurls" => "gau",
            "katana" => "gau",
            "subfinder" => "assetfinder",
            "amass" => "subfinder",
            "assetfinder" => "subfinder",
            "dnsx" => "subfinder",
            "httpx" => "curl",
            "nuclei" => "httpx",
            _ => return tool.to_string(),
        };

        if let ToolStatus::Present { .. } = which(alternative) {
            tracing::info!(
                "Tool '{}' not found, using alternative '{}'",
                tool,
                alternative
            );
            return alternative.to_string();
        }

        // No alternative available, return original (will fail later)
        tracing::warn!("Tool '{}' not found and no alternative available", tool);
        tool.to_string()
    }

    /// Ensure all required tools are installed, auto-installing if possible.
    pub async fn ensure_tools_installed(&self) {
        use crate::alphacode_app_core::bugbounty_install::{
            ensure_go_installed, install_one, plan_for,
        };

        // First ensure Go is installed
        ensure_go_installed().await;

        // Get list of tools used by tasks
        let tasks = self.tasks.read().await;
        let tools: HashSet<String> = tasks.values().map(|t| t.tool.clone()).collect();
        drop(tasks);

        // Try to install each tool
        for tool in tools {
            if let Some(step) = plan_for(&tool) {
                let outcome = install_one(&step).await;
                match outcome {
                    crate::alphacode_app_core::bugbounty_install::InstallOutcome::AlreadyPresent { .. } => {}
                    crate::alphacode_app_core::bugbounty_install::InstallOutcome::Installed { .. } => {
                        tracing::info!("Auto-installed tool: {}", tool);
                    }
                    crate::alphacode_app_core::bugbounty_install::InstallOutcome::InstalledButNotOnPath { hint, .. } => {
                        tracing::warn!("Tool {} installed but not on PATH: {}", tool, hint);
                    }
                    crate::alphacode_app_core::bugbounty_install::InstallOutcome::Failed { output } => {
                        tracing::warn!("Failed to install tool {}: {}", tool, output);
                    }
                    crate::alphacode_app_core::bugbounty_install::InstallOutcome::MissingPrerequisite { needed, hint } => {
                        tracing::warn!("Cannot install tool {}: missing prerequisite {}: {}", tool, needed, hint);
                    }
                    crate::alphacode_app_core::bugbounty_install::InstallOutcome::Skipped { reason } => {
                        tracing::debug!("Skipped installing tool {}: {}", tool, reason);
                    }
                    crate::alphacode_app_core::bugbounty_install::InstallOutcome::TimedOut => {
                        tracing::warn!("Timed out installing tool: {}", tool);
                    }
                }
            }
        }
    }

    /// Initialize the orchestrator with default tasks for the target.
    pub async fn initialize(&self) -> Result<()> {
        let target = self.target.clone();
        let mut tasks = self.tasks.write().await;

        // Phase 1: Reconnaissance
        tasks.insert(
            "recon_subfinder".to_string(),
            BountyTask {
                id: "recon_subfinder".to_string(),
                name: "Subdomain Enumeration (subfinder)".to_string(),
                description: "Passive subdomain enumeration using subfinder".to_string(),
                tool: "subfinder".to_string(),
                params: serde_json::json!({"domain": target, "all": true}),
                phase: BountyPhase::Reconnaissance,
                completed: false,
                result: None,
                dependencies: vec![],
                priority: 1,
                estimated_duration: 120,
                retry_count: 0,
                max_retries: 3,
            },
        );

        tasks.insert(
            "recon_amass".to_string(),
            BountyTask {
                id: "recon_amass".to_string(),
                name: "Subdomain Enumeration (amass)".to_string(),
                description: "In-depth subdomain enumeration using amass".to_string(),
                tool: "amass".to_string(),
                params: serde_json::json!({"domain": target, "passive": true}),
                phase: BountyPhase::Reconnaissance,
                completed: false,
                result: None,
                dependencies: vec![],
                priority: 2,
                estimated_duration: 300,
                retry_count: 0,
                max_retries: 3,
            },
        );

        tasks.insert(
            "recon_assetfinder".to_string(),
            BountyTask {
                id: "recon_assetfinder".to_string(),
                name: "Asset Discovery (assetfinder)".to_string(),
                description: "Find related domains and subdomains".to_string(),
                tool: "assetfinder".to_string(),
                params: serde_json::json!({"domain": target}),
                phase: BountyPhase::Reconnaissance,
                completed: false,
                result: None,
                dependencies: vec![],
                priority: 3,
                estimated_duration: 60,
                retry_count: 0,
                max_retries: 3,
            },
        );

        tasks.insert(
            "recon_dnsx".to_string(),
            BountyTask {
                id: "recon_dnsx".to_string(),
                name: "DNS Enumeration (dnsx)".to_string(),
                description: "DNS record enumeration and resolution".to_string(),
                tool: "dnsx".to_string(),
                params: serde_json::json!({"domain": target}),
                phase: BountyPhase::Reconnaissance,
                completed: false,
                result: None,
                dependencies: vec![],
                priority: 4,
                estimated_duration: 60,
                retry_count: 0,
                max_retries: 3,
            },
        );

        // Phase 2: Attack Surface Mapping
        tasks.insert(
            "surface_httpx".to_string(),
            BountyTask {
                id: "surface_httpx".to_string(),
                name: "HTTP Probing (httpx)".to_string(),
                description: "Probe discovered subdomains for live HTTP services".to_string(),
                tool: "httpx".to_string(),
                params: serde_json::json!({"targets": [], "title": true, "tech_detect": true, "status_codes": true}),
                phase: BountyPhase::AttackSurfaceMapping,
                completed: false,
                result: None,
                dependencies: vec!["recon_subfinder".to_string(), "recon_amass".to_string()],
                priority: 1,
                estimated_duration: 120,
                retry_count: 0,
                max_retries: 3,
            },
        );

        tasks.insert(
            "surface_katana".to_string(),
            BountyTask {
                id: "surface_katana".to_string(),
                name: "Web Crawling (katana)".to_string(),
                description: "Crawl live hosts to discover URLs and endpoints".to_string(),
                tool: "katana".to_string(),
                params: serde_json::json!({"url": [], "depth": 3, "js_crawl": true}),
                phase: BountyPhase::AttackSurfaceMapping,
                completed: false,
                result: None,
                dependencies: vec!["surface_httpx".to_string()],
                priority: 2,
                estimated_duration: 300,
                retry_count: 0,
                max_retries: 3,
            },
        );

        tasks.insert(
            "surface_gau".to_string(),
            BountyTask {
                id: "surface_gau".to_string(),
                name: "URL Discovery (gau)".to_string(),
                description: "Get all URLs from Wayback Machine, Common Crawl, OTX".to_string(),
                tool: "gau".to_string(),
                params: serde_json::json!({"domain": target}),
                phase: BountyPhase::AttackSurfaceMapping,
                completed: false,
                result: None,
                dependencies: vec![],
                priority: 3,
                estimated_duration: 120,
                retry_count: 0,
                max_retries: 3,
            },
        );

        tasks.insert(
            "surface_waybackurls".to_string(),
            BountyTask {
                id: "surface_waybackurls".to_string(),
                name: "Wayback URLs (waybackurls)".to_string(),
                description: "Pull URLs from Wayback Machine".to_string(),
                tool: "waybackurls".to_string(),
                params: serde_json::json!({"domain": target}),
                phase: BountyPhase::AttackSurfaceMapping,
                completed: false,
                result: None,
                dependencies: vec![],
                priority: 4,
                estimated_duration: 60,
                retry_count: 0,
                max_retries: 3,
            },
        );

        tasks.insert(
            "surface_feroxbuster".to_string(),
            BountyTask {
                id: "surface_feroxbuster".to_string(),
                name: "Content Discovery (feroxbuster)".to_string(),
                description: "Discover hidden directories and files".to_string(),
                tool: "feroxbuster".to_string(),
                params: serde_json::json!({"url": [], "recursive": true, "depth": 3}),
                phase: BountyPhase::AttackSurfaceMapping,
                completed: false,
                result: None,
                dependencies: vec!["surface_httpx".to_string()],
                priority: 5,
                estimated_duration: 300,
                retry_count: 0,
                max_retries: 3,
            },
        );

        // Phase 3: Vulnerability Discovery
        tasks.insert(
            "vuln_nuclei".to_string(),
            BountyTask {
                id: "vuln_nuclei".to_string(),
                name: "Template Scanning (nuclei)".to_string(),
                description: "Scan for known vulnerabilities using nuclei templates".to_string(),
                tool: "nuclei".to_string(),
                params: serde_json::json!({"target": [], "severity": "critical,high,medium"}),
                phase: BountyPhase::VulnerabilityDiscovery,
                completed: false,
                result: None,
                dependencies: vec!["surface_httpx".to_string()],
                priority: 1,
                estimated_duration: 300,
                retry_count: 0,
                max_retries: 3,
            },
        );

        tasks.insert(
            "vuln_nikto".to_string(),
            BountyTask {
                id: "vuln_nikto".to_string(),
                name: "Web Server Scanning (nikto)".to_string(),
                description: "Scan for known web server vulnerabilities".to_string(),
                tool: "nikto".to_string(),
                params: serde_json::json!({"url": []}),
                phase: BountyPhase::VulnerabilityDiscovery,
                completed: false,
                result: None,
                dependencies: vec!["surface_httpx".to_string()],
                priority: 2,
                estimated_duration: 300,
                retry_count: 0,
                max_retries: 3,
            },
        );

        tasks.insert(
            "vuln_dalfox".to_string(),
            BountyTask {
                id: "vuln_dalfox".to_string(),
                name: "XSS Scanning (dalfox)".to_string(),
                description: "Scan for XSS vulnerabilities".to_string(),
                tool: "dalfox".to_string(),
                params: serde_json::json!({"url": []}),
                phase: BountyPhase::VulnerabilityDiscovery,
                completed: false,
                result: None,
                dependencies: vec!["surface_katana".to_string()],
                priority: 3,
                estimated_duration: 300,
                retry_count: 0,
                max_retries: 3,
            },
        );

        tasks.insert(
            "vuln_corsy".to_string(),
            BountyTask {
                id: "vuln_corsy".to_string(),
                name: "CORS Scanning (corsy)".to_string(),
                description: "Scan for CORS misconfigurations".to_string(),
                tool: "corsy".to_string(),
                params: serde_json::json!({"url": []}),
                phase: BountyPhase::VulnerabilityDiscovery,
                completed: false,
                result: None,
                dependencies: vec!["surface_httpx".to_string()],
                priority: 4,
                estimated_duration: 120,
                retry_count: 0,
                max_retries: 3,
            },
        );

        tasks.insert(
            "vuln_crlfuzz".to_string(),
            BountyTask {
                id: "vuln_crlfuzz".to_string(),
                name: "CRLF Scanning (crlfuzz)".to_string(),
                description: "Scan for CRLF injection vulnerabilities".to_string(),
                tool: "crlfuzz".to_string(),
                params: serde_json::json!({"url": []}),
                phase: BountyPhase::VulnerabilityDiscovery,
                completed: false,
                result: None,
                dependencies: vec!["surface_httpx".to_string()],
                priority: 5,
                estimated_duration: 120,
                retry_count: 0,
                max_retries: 3,
            },
        );

        // Phase 4: Exploitation
        tasks.insert(
            "exploit_sqlmap".to_string(),
            BountyTask {
                id: "exploit_sqlmap".to_string(),
                name: "SQL Injection (sqlmap)".to_string(),
                description: "Test for SQL injection vulnerabilities".to_string(),
                tool: "sqlmap".to_string(),
                params: serde_json::json!({"url": [], "batch": true, "level": 2, "risk": 2}),
                phase: BountyPhase::Exploitation,
                completed: false,
                result: None,
                dependencies: vec!["surface_katana".to_string()],
                priority: 1,
                estimated_duration: 600,
                retry_count: 0,
                max_retries: 3,
            },
        );

        tasks.insert(
            "exploit_gobuster".to_string(),
            BountyTask {
                id: "exploit_gobuster".to_string(),
                name: "Directory Bruteforce (gobuster)".to_string(),
                description: "Brute force directories and files".to_string(),
                tool: "gobuster".to_string(),
                params: serde_json::json!({"url": [], "mode": "dir"}),
                phase: BountyPhase::Exploitation,
                completed: false,
                result: None,
                dependencies: vec!["surface_httpx".to_string()],
                priority: 2,
                estimated_duration: 300,
                retry_count: 0,
                max_retries: 3,
            },
        );

        Ok(())
    }

    /// Get all tasks for a specific phase.
    pub async fn get_tasks_for_phase(&self, phase: BountyPhase) -> Vec<BountyTask> {
        let tasks = self.tasks.read().await;
        tasks
            .values()
            .filter(|t| t.phase == phase)
            .cloned()
            .collect()
    }

    /// Get all pending tasks.
    pub async fn get_pending_tasks(&self) -> Vec<BountyTask> {
        let tasks = self.tasks.read().await;
        tasks.values().filter(|t| !t.completed).cloned().collect()
    }

    /// Get all completed tasks.
    pub async fn get_completed_tasks(&self) -> Vec<BountyTask> {
        let tasks = self.tasks.read().await;
        tasks.values().filter(|t| t.completed).cloned().collect()
    }

    /// Mark a task as completed.
    pub async fn complete_task(&self, task_id: &str, result: String) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(task_id) {
            task.completed = true;
            task.result = Some(result);
        }
        Ok(())
    }

    /// Get the current phase.
    pub async fn get_current_phase(&self) -> BountyPhase {
        *self.current_phase.read().await
    }

    /// Advance to the next phase.
    pub async fn advance_phase(&self) -> Result<()> {
        let mut phase = self.current_phase.write().await;
        if let Some(next) = phase.next() {
            *phase = next;
        }
        Ok(())
    }

    /// Get a summary of the orchestrator state.
    pub async fn get_summary(&self) -> OrchestratorSummary {
        let tasks = self.tasks.read().await;
        let subdomains = self.subdomains.read().await;
        let urls = self.urls.read().await;
        let vulnerabilities = self.vulnerabilities.read().await;
        let ports = self.ports.read().await;
        let services = self.services.read().await;
        let current_phase = *self.current_phase.read().await;

        let total_tasks = tasks.len();
        let completed_tasks = tasks.values().filter(|t| t.completed).count();
        let pending_tasks = total_tasks - completed_tasks;

        OrchestratorSummary {
            target: self.target.clone(),
            current_phase,
            total_tasks,
            completed_tasks,
            pending_tasks,
            subdomains_found: subdomains.len(),
            urls_found: urls.len(),
            vulnerabilities_found: vulnerabilities.len(),
            ports_found: ports.len(),
            services_found: services.len(),
        }
    }
}

/// Summary of the orchestrator state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorSummary {
    /// The target domain or URL
    pub target: String,
    /// Current phase
    pub current_phase: BountyPhase,
    /// Total number of tasks
    pub total_tasks: usize,
    /// Number of completed tasks
    pub completed_tasks: usize,
    /// Number of pending tasks
    pub pending_tasks: usize,
    /// Number of subdomains found
    pub subdomains_found: usize,
    /// Number of URLs found
    pub urls_found: usize,
    /// Number of vulnerabilities found
    pub vulnerabilities_found: usize,
    /// Number of ports found
    pub ports_found: usize,
    /// Number of services found
    pub services_found: usize,
}

impl std::fmt::Display for OrchestratorSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Bug Bounty Orchestrator Summary")?;
        writeln!(f, "===============================")?;
        writeln!(f, "Target: {}", self.target)?;
        writeln!(f, "Phase: {:?}", self.current_phase)?;
        writeln!(
            f,
            "Tasks: {}/{} completed",
            self.completed_tasks, self.total_tasks
        )?;
        writeln!(f, "Subdomains: {}", self.subdomains_found)?;
        writeln!(f, "URLs: {}", self.urls_found)?;
        writeln!(f, "Vulnerabilities: {}", self.vulnerabilities_found)?;
        writeln!(f, "Ports: {}", self.ports_found)?;
        writeln!(f, "Services: {}", self.services_found)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let orchestrator = BugBountyOrchestrator::new("example.com");
        assert_eq!(orchestrator.target, "example.com");
    }

    #[tokio::test]
    async fn test_orchestrator_initialize() {
        let orchestrator = BugBountyOrchestrator::new("example.com");
        orchestrator.initialize().await.unwrap();
        let tasks = orchestrator.get_pending_tasks().await;
        assert!(!tasks.is_empty());
    }

    #[tokio::test]
    async fn test_phase_advancement() {
        let orchestrator = BugBountyOrchestrator::new("example.com");
        let phase = orchestrator.get_current_phase().await;
        assert_eq!(phase, BountyPhase::Reconnaissance);
        orchestrator.advance_phase().await.unwrap();
        let phase = orchestrator.get_current_phase().await;
        assert_eq!(phase, BountyPhase::AttackSurfaceMapping);
    }

    #[tokio::test]
    async fn test_task_completion() {
        let orchestrator = BugBountyOrchestrator::new("example.com");
        orchestrator.initialize().await.unwrap();
        orchestrator
            .complete_task("recon_subfinder", "Found 10 subdomains".to_string())
            .await
            .unwrap();
        let completed = orchestrator.get_completed_tasks().await;
        assert_eq!(completed.len(), 1);
    }
}
