use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessProfile {
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModulesFlags {
    pub employees: bool,
    pub appointments: bool,
    pub services: bool,
    pub packages: bool,
    pub reports: bool,
}

impl ModulesFlags {
    /// Reglas de dependencia (puerto de src/lib/modules.ts `normalizeModules`),
    /// sin el chequeo de plan que existía en la versión SaaS.
    pub fn normalize(mut self) -> Self {
        if self.appointments {
            self.employees = true;
        }
        if !self.employees {
            self.appointments = false;
        }
        self
    }
}
