use serde::{Deserialize, Serialize};

/// A reusable knowledge entry extracted from live analysis.
///
/// Everything here is discovered at runtime from the target.
/// No vulnerability patterns, payloads, or techniques are pre-populated.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub id: String,
    pub title: String,
    pub vulnerability_class: String,
    pub root_cause: String,
    pub attack_preconditions: Vec<String>,
    pub request_pattern: Option<String>,
    pub response_pattern: Option<String>,
    pub technology: Option<String>,
    pub bypass_techniques: Vec<String>,
    pub edge_cases: Vec<String>,
    pub validation_method: String,
    pub impact: String,
    pub defensive_lesson: String,
    pub tags: Vec<String>,
    pub source: Option<String>,
    pub confidence: f64,
    // Enhanced knowledge fields
    pub cve_references: Vec<String>,
    pub owasp_category: Option<String>,
    pub cvss_score: Option<f64>,
    pub cvss_vector: Option<String>,
    pub affected_versions: Vec<String>,
    pub patched_versions: Vec<String>,
    pub exploit_complexity: Option<String>,
    pub exploit_availability: Option<String>,
    pub remediation_steps: Vec<String>,
    pub detection_methods: Vec<String>,
    pub false_positive_indicators: Vec<String>,
    pub related_cwes: Vec<String>,
    pub attack_chain_position: Option<String>,
    pub prerequisites: Vec<String>,
    pub postconditions: Vec<String>,
    pub indicators_of_compromise: Vec<String>,
    pub mitigation_strategies: Vec<String>,
    pub testing_approach: String,
    pub automation_potential: f64,
    pub false_positive_rate: f64,
    pub true_positive_rate: f64,
    pub average_time_to_exploit: Option<String>,
    pub skill_required: Option<String>,
    pub tools_required: Vec<String>,
    pub references: Vec<String>,
    pub notes: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub version: u32,
}

impl KnowledgeEntry {
    pub fn new(id: String, title: String, vuln_class: String, root_cause: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            title,
            vulnerability_class: vuln_class,
            root_cause,
            attack_preconditions: Vec::new(),
            request_pattern: None,
            response_pattern: None,
            technology: None,
            bypass_techniques: Vec::new(),
            edge_cases: Vec::new(),
            validation_method: String::new(),
            impact: String::new(),
            defensive_lesson: String::new(),
            tags: Vec::new(),
            source: None,
            confidence: 0.5,
            // Enhanced knowledge fields
            cve_references: Vec::new(),
            owasp_category: None,
            cvss_score: None,
            cvss_vector: None,
            affected_versions: Vec::new(),
            patched_versions: Vec::new(),
            exploit_complexity: None,
            exploit_availability: None,
            remediation_steps: Vec::new(),
            detection_methods: Vec::new(),
            false_positive_indicators: Vec::new(),
            related_cwes: Vec::new(),
            attack_chain_position: None,
            prerequisites: Vec::new(),
            postconditions: Vec::new(),
            indicators_of_compromise: Vec::new(),
            mitigation_strategies: Vec::new(),
            testing_approach: String::new(),
            automation_potential: 0.5,
            false_positive_rate: 0.0,
            true_positive_rate: 0.0,
            average_time_to_exploit: None,
            skill_required: None,
            tools_required: Vec::new(),
            references: Vec::new(),
            notes: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
            version: 1,
        }
    }

    pub fn is_relevant(&self, tech: &str, vuln_class: &str) -> bool {
        let tech_match = self
            .technology
            .as_ref()
            .map(|t| t.eq_ignore_ascii_case(tech))
            .unwrap_or(true);
        let class_match = self.vulnerability_class.eq_ignore_ascii_case(vuln_class);
        tech_match && class_match
    }

    /// Update the knowledge entry with new information.
    pub fn update(&mut self, other: &KnowledgeEntry) {
        // Merge vectors without duplicates
        for item in &other.attack_preconditions {
            if !self.attack_preconditions.contains(item) {
                self.attack_preconditions.push(item.clone());
            }
        }
        for item in &other.bypass_techniques {
            if !self.bypass_techniques.contains(item) {
                self.bypass_techniques.push(item.clone());
            }
        }
        for item in &other.edge_cases {
            if !self.edge_cases.contains(item) {
                self.edge_cases.push(item.clone());
            }
        }
        for item in &other.tags {
            if !self.tags.contains(item) {
                self.tags.push(item.clone());
            }
        }
        for item in &other.cve_references {
            if !self.cve_references.contains(item) {
                self.cve_references.push(item.clone());
            }
        }
        for item in &other.remediation_steps {
            if !self.remediation_steps.contains(item) {
                self.remediation_steps.push(item.clone());
            }
        }
        for item in &other.detection_methods {
            if !self.detection_methods.contains(item) {
                self.detection_methods.push(item.clone());
            }
        }
        for item in &other.false_positive_indicators {
            if !self.false_positive_indicators.contains(item) {
                self.false_positive_indicators.push(item.clone());
            }
        }
        for item in &other.mitigation_strategies {
            if !self.mitigation_strategies.contains(item) {
                self.mitigation_strategies.push(item.clone());
            }
        }
        for item in &other.indicators_of_compromise {
            if !self.indicators_of_compromise.contains(item) {
                self.indicators_of_compromise.push(item.clone());
            }
        }
        for item in &other.tools_required {
            if !self.tools_required.contains(item) {
                self.tools_required.push(item.clone());
            }
        }
        for item in &other.references {
            if !self.references.contains(item) {
                self.references.push(item.clone());
            }
        }
        for item in &other.notes {
            if !self.notes.contains(item) {
                self.notes.push(item.clone());
            }
        }

        // Update scalar fields if they are more specific
        if other.confidence > self.confidence {
            self.confidence = other.confidence;
        }
        if other.cvss_score.is_some() {
            self.cvss_score = other.cvss_score;
        }
        if other.cvss_vector.is_some() {
            self.cvss_vector = other.cvss_vector.clone();
        }
        if other.owasp_category.is_some() {
            self.owasp_category = other.owasp_category.clone();
        }
        if !other.impact.is_empty() {
            self.impact = other.impact.clone();
        }
        if !other.defensive_lesson.is_empty() {
            self.defensive_lesson = other.defensive_lesson.clone();
        }
        if !other.validation_method.is_empty() {
            self.validation_method = other.validation_method.clone();
        }
        if !other.testing_approach.is_empty() {
            self.testing_approach = other.testing_approach.clone();
        }
        if other.automation_potential > self.automation_potential {
            self.automation_potential = other.automation_potential;
        }

        self.updated_at = chrono::Utc::now().to_rfc3339();
        self.version += 1;
    }

    /// Calculate the overall quality score of this knowledge entry in
    /// `0.0..=1.0`.
    ///
    /// The weights (0.3 + 0.2 + 0.3 + 0.2) sum to 1.0, so the score is their
    /// weighted sum. This used to divide by a `factors` counter that counted
    /// 3 or 4 regardless of what was present, capping the result at ~0.27 —
    /// which made [`Self::is_high_quality`]'s `>= 0.7` test unsatisfiable, so
    /// it returned `false` for every possible entry.
    pub fn quality_score(&self) -> f64 {
        let mut score = 0.0;

        // Confidence contributes up to 0.3
        score += self.confidence * 0.3;

        // CVSS score contributes up to 0.2
        if let Some(cvss) = self.cvss_score {
            score += (cvss / 10.0).clamp(0.0, 1.0) * 0.2;
        }

        // Completeness of information contributes up to 0.3
        let completeness = [
            !self.attack_preconditions.is_empty(),
            self.request_pattern.is_some(),
            self.response_pattern.is_some(),
            !self.bypass_techniques.is_empty(),
            !self.remediation_steps.is_empty(),
            !self.detection_methods.is_empty(),
            !self.mitigation_strategies.is_empty(),
        ]
        .iter()
        .filter(|&&x| x)
        .count() as f64
            / 7.0;
        score += completeness * 0.3;

        // Automation potential contributes up to 0.2
        score += self.automation_potential * 0.2;

        score.clamp(0.0, 1.0)
    }

    /// Check if this knowledge entry is high quality enough to be reused.
    pub fn is_high_quality(&self) -> bool {
        self.quality_score() >= 0.7 && self.confidence >= 0.8
    }
}

/// A knowledge store populated entirely from live analysis.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct KnowledgeStore {
    pub entries: Vec<KnowledgeEntry>,
}

impl KnowledgeStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, entry: KnowledgeEntry) {
        if !self.entries.iter().any(|e| e.id == entry.id) {
            self.entries.push(entry);
        }
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.id != id);
        self.entries.len() < before
    }

    /// Lookup by class, optionally filtered by technology.
    ///
    /// `technology=None` matches everything. `Some(t)` matches entries for
    /// technology `t` PLUS generic entries with no technology set (wildcards).
    pub fn lookup(&self, vuln_class: &str, technology: Option<&str>) -> Vec<&KnowledgeEntry> {
        self.entries
            .iter()
            .filter(|e| {
                let class_match = e.vulnerability_class.eq_ignore_ascii_case(vuln_class);
                let tech_match = technology
                    .map(|t| {
                        e.technology
                            .as_ref()
                            .map(|et| et.eq_ignore_ascii_case(t))
                            .unwrap_or(true)
                    })
                    .unwrap_or(true);
                class_match && tech_match
            })
            .collect()
    }

    /// Strict technology filter: excludes generic entries with no technology set.
    /// Use [`lookup`](Self::lookup) when generic entries should be included.
    pub fn by_technology(&self, tech: &str) -> Vec<&KnowledgeEntry> {
        self.entries
            .iter()
            .filter(|e| {
                e.technology
                    .as_ref()
                    .map(|t| t.eq_ignore_ascii_case(tech))
                    .unwrap_or(false)
            })
            .collect()
    }

    pub fn by_vuln_class(&self, class: &str) -> Vec<&KnowledgeEntry> {
        self.entries
            .iter()
            .filter(|e| e.vulnerability_class.eq_ignore_ascii_case(class))
            .collect()
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }

    pub fn merge(&mut self, other: KnowledgeStore) {
        for entry in other.entries {
            self.add(entry);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knowledge_store_add_and_lookup() {
        let mut store = KnowledgeStore::new();
        let entry = KnowledgeEntry::new(
            "k1".into(),
            "Test finding".into(),
            "xss".into(),
            "unescaped output".into(),
        );
        store.add(entry);
        assert_eq!(store.count(), 1);

        let results = store.lookup("xss", None);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn knowledge_store_deduplicates() {
        let mut store = KnowledgeStore::new();
        store.add(KnowledgeEntry::new(
            "k1".into(),
            "A".into(),
            "xss".into(),
            "rc".into(),
        ));
        store.add(KnowledgeEntry::new(
            "k1".into(),
            "B".into(),
            "sqli".into(),
            "rc".into(),
        ));
        assert_eq!(store.count(), 1);
    }

    #[test]
    fn knowledge_store_remove() {
        let mut store = KnowledgeStore::new();
        store.add(KnowledgeEntry::new(
            "k1".into(),
            "A".into(),
            "xss".into(),
            "rc".into(),
        ));
        assert!(store.remove("k1"));
        assert_eq!(store.count(), 0);
        assert!(!store.remove("k1"));
    }

    #[test]
    fn knowledge_store_merge() {
        let mut a = KnowledgeStore::new();
        a.add(KnowledgeEntry::new(
            "k1".into(),
            "A".into(),
            "xss".into(),
            "rc".into(),
        ));
        let mut b = KnowledgeStore::new();
        b.add(KnowledgeEntry::new(
            "k2".into(),
            "B".into(),
            "sqli".into(),
            "rc".into(),
        ));
        b.add(KnowledgeEntry::new(
            "k1".into(),
            "C".into(),
            "xss".into(),
            "rc2".into(),
        ));
        a.merge(b);
        assert_eq!(a.count(), 2);
    }

    #[test]
    fn knowledge_by_technology_filters() {
        let mut store = KnowledgeStore::new();
        store.add(KnowledgeEntry {
            technology: Some("react".into()),
            ..KnowledgeEntry::new("k1".into(), "A".into(), "xss".into(), "rc".into())
        });
        store.add(KnowledgeEntry {
            technology: None,
            ..KnowledgeEntry::new("k2".into(), "B".into(), "xss".into(), "rc".into())
        });
        let react = store.by_technology("react");
        assert_eq!(react.len(), 1);
        let all = store.by_vuln_class("xss");
        assert_eq!(all.len(), 2);
    }
}
